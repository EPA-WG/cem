use super::*;
use cem_ql::template_execution_record::browser::RecordedOutputSession;
fn request() -> String {
    serde_json::json!({
        "policyStamp":"base:1", "browser":{"engine":"chromium","version":CHROMIUM_BASE_PROFILE_VERSION},
        "rootUri": ROOT, "outputUri": OUTPUT, "resolutionBase": OUTPUT,
        "sources": [{"uri":ROOT,"revision":"1","source":ROOT_SOURCE,"contentType":"text/cem-ml"},
                    {"uri":CHILD,"revision":"1","source":CHILD_SOURCE.replace(" layer(base)", ""),"contentType":"text/cem-ml"}],
        "stylesheets": css_sources_input(),
        "choices": [{"bodyUri":ROOT,"index":0,"scope":{"kind":"private","tag":"cem-card"}},
                    {"bodyUri":CHILD,"index":0,"scope":{"kind":"private","tag":"cem-card"}}]
    }).to_string()
}
fn css_sources_input() -> Vec<serde_json::Value> {
    // This fixture uses no external CSS; replace body rules before construction.
    vec![]
}
fn session() -> RecordedOutputSession {
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    for source in value["sources"].as_array_mut().unwrap() {
        let text = source["source"].as_str().unwrap();
        source["source"] = text
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .replace(" @style='background:url(./document.svg)'", "")
            .into();
    }
    value["choices"] = serde_json::json!([]);
    RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap()
}
fn compose(session: &mut RecordedOutputSession) {
    session
        .prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    session.compose(BASE, slots).unwrap();
}
#[test]
fn staged_native_session_owns_sources_through_delivery_and_capture() {
    let mut s = session();
    assert!(s.html().unwrap().contains("<html"));
    compose(&mut s);
    assert!(s.html().unwrap().contains("<html"));
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    let blockers = s
        .capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(blockers
        .iter()
        .any(|b| b == "atomic-host-transaction-required"));
    assert_eq!(
        blockers,
        s.read(BASE, slots, &mut OutputObserver { wrong: false })
            .unwrap()
    );
    s.close();
    assert!(s.html().is_err());
    assert!(s
        .read(BASE, slots, &mut OutputObserver { wrong: false })
        .is_err());
}
#[test]
fn out_of_order_and_failed_session_calls_latch_closed() {
    let mut s = session();
    assert!(s.compose(BASE, slots).is_err());
    assert!(s
        .prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .is_err());
    let mut s = session();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(s
        .read(
            BASE,
            |_| Err("DOM changed".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert!(s
        .read(BASE, slots, &mut OutputObserver { wrong: false })
        .is_err());
}
#[test]
fn session_rejects_unbounded_or_unknown_source_envelopes() {
    assert!(RecordedOutputSession::from_json(&" ".repeat(4 * 1024 * 1024 + 1), 1, 1).is_err());
    assert!(RecordedOutputSession::from_json("{\"ast\":{}}", 1, 1).is_err());
    assert!(RecordedOutputSession::from_json(&request(), 0, 1).is_err());
}

#[test]
fn staged_session_composes_native_styles_and_preserves_incomplete_resource_blockers() {
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    for source in value["sources"].as_array_mut().unwrap() {
        let body = source["source"]
            .as_str()
            .unwrap()
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .replace(" @style='background:url(./document.svg)'", "");
        source["source"] = format!("{{style |```p{{color:red}}```}}\n{body}").into();
    }
    let mut s = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    compose(&mut s);
    assert!(s.html().unwrap().contains("@scope"));
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    let result = s
        .capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(result
        .iter()
        .any(|b| b == "css-runtime-dependency-closure-required"));
}

#[test]
fn staged_session_denial_and_callback_cancellation_cannot_publish() {
    let mut s = session();
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Deny))
        .unwrap();
    assert!(s.compose(BASE, slots).is_err());
    assert!(s.html().is_err());
    let mut s = session();
    let cancel = s.cancellation();
    assert!(s
        .prepare_base(
            |slot| {
                cancel.abort();
                slots(slot)
            },
            |_| Ok(HtmlBasePolicyDecision::Allow)
        )
        .is_err());
    assert!(s.html().is_err());
}

#[test]
fn missing_acquired_css_cannot_fall_back_to_network() {
    let mut s = RecordedOutputSession::from_json(&request(), 1, 1).unwrap();
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    assert!(s.compose(BASE, slots).is_err());
    assert!(s.html().is_err());
}

#[test]
fn staged_stylesheet_projection_and_selection_retain_native_acquisition() {
    let mut s = session();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    let (sources, actions) = s.prepare_stylesheets(BASE, slots, Some("")).unwrap();
    assert!(sources.is_empty() && actions.is_empty());
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(s
        .read(BASE, slots, &mut OutputObserver { wrong: false })
        .is_ok());
}

#[test]
fn staged_selection_rejects_unknown_titles_and_latches_closed() {
    let mut s = session();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    assert!(s.prepare_stylesheets(BASE, slots, Some("missing")).is_err());
    assert!(s.html().is_err());
}

#[test]
fn external_titled_media_sources_and_commands_are_native_owned() {
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    for source in value["sources"].as_array_mut().unwrap() {
        let body = source["source"].as_str().unwrap().lines().skip(1).collect::<Vec<_>>().join("\n")
            .replace(" @style='background:url(./document.svg)'", "")
            .replace("{head |", "{head | {link @rel='alternate stylesheet' @href='theme.css' @title=night @media=screen}");
        source["source"] = body.into();
    }
    value["choices"] = serde_json::json!([]);
    value["stylesheets"] = serde_json::json!([
        {"requestedUrl":format!("{BASE}theme.css"),"finalUrl":format!("{BASE}theme.css"),
         "revision":"1","source":"@import 'child.css' screen; p{color:red}","contentType":"text/css"},
        {"requestedUrl":format!("{BASE}child.css"),"finalUrl":format!("{BASE}child.css"),
         "revision":"1","source":"p{font-style:italic}","contentType":"text/css"}
    ]);
    let mut s = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    let (sources, actions) = s.prepare_stylesheets(BASE, slots, Some("night")).unwrap();
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].url, format!("{BASE}theme.css"));
    assert_eq!(sources[1].parent_sheet, Some(0));
    assert_eq!(sources[1].import_index, Some(0));
    assert_eq!(sources[1].text, "p{font-style:italic}");
    assert_eq!(actions.len(), 1);
    assert!(actions[0].link && actions[0].selected);
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(s
        .read(BASE, slots, &mut OutputObserver { wrong: false })
        .is_ok());
    // Repeated source/selection preparation cannot mutate an existing record.
    assert!(s.prepare_stylesheets(BASE, slots, Some("")).is_err());
    assert!(s.html().is_err());
    value["stylesheets"] = serde_json::json!([]);
    let mut missing = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    missing
        .prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    assert!(missing.compose(BASE, slots).is_err());
}

#[test]
fn staged_images_keep_acquisition_with_output_and_revoke_on_dom_failure() {
    use cem_ml::html_image_execution::{ImageObservation, ImageResponse};
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    for source in value["sources"].as_array_mut().unwrap() {
        source["source"] = source["source"]
            .as_str()
            .unwrap()
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .replace(
                "{p @style='background:url(./document.svg)'",
                "{img @src='a.png'} {p",
            )
            .into();
    }
    value["choices"] = serde_json::json!([]);
    let mut s = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    s.prepare_stylesheets(BASE, slots, None).unwrap();
    let observation = ImageObservation {
        url: format!("{BASE}a.png"),
        width: 1,
        height: 1,
        complete: true,
    };
    let selected = s.select_images(|_| Ok(observation.clone())).unwrap();
    assert_eq!(selected.len(), 1);
    s.record_images(
        vec![ImageResponse {
            requested_url: observation.url.clone(),
            final_url: observation.url.clone(),
            revision: "1".into(),
            content_type: "image/png".into(),
            bytes: vec![1],
        }],
        "env",
        |_| Ok(observation.clone()),
    )
    .unwrap();
    assert_eq!(s.image_acquisitions(), 1);
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    s.validate_images("env", |_| Ok(observation.clone()))
        .unwrap();
    assert!(s
        .read(
            BASE,
            |_| Err("DOM changed".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert!(s
        .validate_images("env", |_| Ok(observation.clone()))
        .is_err());
    assert_eq!(s.image_acquisitions(), 0);
}

#[test]
fn staged_image_delivery_is_joined_and_revoked_with_output() {
    use cem_ml::html_image_execution::{ImageObservation, ImageResponse};
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    for source in value["sources"].as_array_mut().unwrap() {
        source["source"] = source["source"]
            .as_str()
            .unwrap()
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .replace(
                "{p @style='background:url(./document.svg)'",
                "{img @src='a.png'} {p",
            )
            .into();
    }
    value["choices"] = serde_json::json!([]);
    let mut s = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    s.begin_image_delivery().unwrap();
    s.accept_image_delivery(ImageResponse {
        requested_url: format!("{BASE}a.png"),
        final_url: format!("{BASE}a.png"),
        revision: String::new(),
        content_type: "image/png".into(),
        bytes: vec![1],
    })
    .unwrap();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    s.prepare_stylesheets(BASE, slots, None).unwrap();
    let observation = ImageObservation {
        url: format!("{BASE}a.png"),
        width: 1,
        height: 1,
        complete: true,
    };
    let selected = s.select_images(|_| Ok(observation.clone())).unwrap();
    assert_eq!(selected.len(), 1);
    s.record_delivered_images("env", |_| Ok(observation.clone()))
        .unwrap();
    assert_eq!(s.image_deliveries(), 1);
    assert_eq!(s.image_acquisitions(), 1);
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    s.validate_images("env", |_| Ok(observation.clone()))
        .unwrap();
    assert!(s
        .read(
            BASE,
            |_| Err("DOM changed".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert!(s
        .validate_images("env", |_| Ok(observation.clone()))
        .is_err());
    assert_eq!(s.image_acquisitions(), 0);
    assert_eq!(s.image_deliveries(), 0);
}

#[test]
fn image_delivery_ingress_cannot_outlive_or_precede_its_session() {
    use cem_ml::html_image_execution::ImageResponse;
    let response = || ImageResponse {
        requested_url: format!("{BASE}a.png"),
        final_url: format!("{BASE}a.png"),
        content_type: "image/png".into(),
        revision: String::new(),
        bytes: vec![1],
    };
    let mut s = session();
    assert!(s.accept_image_delivery(response()).is_err());
    assert!(s.begin_image_delivery().is_err());
    let mut s = session();
    compose(&mut s);
    assert!(s.begin_image_delivery().is_err());
    let mut s = session();
    s.begin_image_delivery().unwrap();
    s.close();
    assert!(s.accept_image_delivery(response()).is_err());
    let mut s = session();
    s.begin_image_delivery().unwrap();
    let mut foreign = response();
    foreign.requested_url = "https://foreign.test/a.png".into();
    foreign.final_url = foreign.requested_url.clone();
    assert!(s.accept_image_delivery(foreign).is_err());
    assert!(s.accept_image_delivery(response()).is_err());
}

#[test]
fn staged_replay_routes_bind_new_delivery_to_retained_output() {
    use cem_ml::html_image_execution::{ImageObservation, ImageResponse};
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    for source in value["sources"].as_array_mut().unwrap() {
        source["source"] = source["source"]
            .as_str()
            .unwrap()
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .replace(
                "{p @style='background:url(./document.svg)'",
                "{img @src='a.png'} {p",
            )
            .into();
    }
    value["choices"] = serde_json::json!([]);
    let mut s = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    s.begin_image_delivery().unwrap();
    s.enable_image_replay(&"a".repeat(32)).unwrap();
    compose(&mut s);
    let routes = s.image_replay_routes().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].1, format!("{BASE}a.png"));
    assert!(s.html().unwrap().contains(&routes[0].0));
    s.accept_image_delivery(ImageResponse {
        requested_url: routes[0].0.clone(),
        final_url: routes[0].0.clone(),
        revision: String::new(),
        content_type: "image/png".into(),
        bytes: vec![1],
    })
    .unwrap();
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    s.prepare_stylesheets(BASE, slots, None).unwrap();
    let observation = ImageObservation {
        url: routes[0].0.clone(),
        width: 1,
        height: 1,
        complete: true,
    };
    let selected = s.select_images(|_| Ok(observation.clone())).unwrap();
    assert_eq!(selected.len(), 1);
    s.record_delivered_images("env", |_| Ok(observation.clone()))
        .unwrap();
    assert_eq!(s.image_deliveries(), 1);
    assert!(s.image_replay_bound());
    assert_eq!(s.image_acquisitions(), 1);
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    s.validate_images("env", |_| Ok(observation.clone()))
        .unwrap();
    assert!(s
        .read(
            BASE,
            |_| Err("DOM changed".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert!(s
        .validate_images("env", |_| Ok(observation.clone()))
        .is_err());
    assert_eq!(s.image_acquisitions(), 0);
    assert_eq!(s.image_deliveries(), 0);
    assert!(!s.image_replay_bound());
}

struct SvgObserver(&'static str);
impl cem_ml::svg_computed_execution::SvgComputedObserver for SvgObserver {
    fn environment_revision(&mut self) -> Result<String, String> {
        Ok("svg:1".into())
    }
    fn observe(
        &mut self,
        slot: cem_ml::svg_computed_execution::SvgComputedSlot<'_>,
    ) -> Result<String, String> {
        assert_eq!(slot.property, "fill");
        Ok(self.0.into())
    }
}
fn svg_session() -> RecordedOutputSession {
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    for source in value["sources"].as_array_mut().unwrap() {
        source["source"] = source["source"].as_str().unwrap().lines().skip(1).collect::<Vec<_>>().join("\n")
            .replace("{p @style='background:url(./document.svg)'", "{result-element @name=svg @namespace='http://www.w3.org/2000/svg' | {result-element @name=path @namespace='http://www.w3.org/2000/svg' | {result-attribute @name=fill | red}}} {p").into();
    }
    value["choices"] = serde_json::json!([]);
    let mut s = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    s.prepare_stylesheets(BASE, slots, None).unwrap();
    s
}
#[test]
fn composed_svg_evidence_is_bound_to_session_lifetime() {
    let mut s = svg_session();
    s.record_svg(&mut SvgObserver("rgb(0, 0, 255)")).unwrap();
    assert!(s.svg_computed_slots() > 0);
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    s.validate_svg(&mut SvgObserver("rgb(0, 0, 255)")).unwrap();
    assert!(s.validate_svg(&mut SvgObserver("rgb(255, 0, 0)")).is_err());
    assert!(s.html().is_err());
    assert!(s.validate_svg(&mut SvgObserver("rgb(0, 0, 255)")).is_err());
}
#[test]
fn svg_capture_requires_preparation_and_cannot_be_repeated() {
    let mut s = session();
    assert!(s.record_svg(&mut SvgObserver("none")).is_err());
    assert!(s.html().is_err());
    let mut s = svg_session();
    s.record_svg(&mut SvgObserver("none")).unwrap();
    assert!(s.record_svg(&mut SvgObserver("none")).is_err());
    assert!(s.html().is_err());
    let mut s = svg_session();
    assert!(s.validate_svg(&mut SvgObserver("none")).is_err());
}

fn paint_session() -> RecordedOutputSession {
    paint_session_with_clone(false)
}
fn paint_session_with_clone(clone: bool) -> RecordedOutputSession {
    let mut value: serde_json::Value = serde_json::from_str(&request()).unwrap();
    value["choices"] = serde_json::json!([]);
    value["sources"] = serde_json::json!([{"uri": ROOT, "revision":"1", "contentType":"text/cem-ml", "source": format!("{{result-element @name=html @namespace='http://www.w3.org/1999/xhtml' | {{result-element @name=head @namespace='http://www.w3.org/1999/xhtml' | {{result-element @name=base @namespace='http://www.w3.org/1999/xhtml' | {{result-attribute @name=href | {BASE}}}}}}}{{result-element @name=body @namespace='http://www.w3.org/1999/xhtml' | {{svg @xmlns='http://www.w3.org/2000/svg' | {{linearGradient @id=a @href='#b' | {{stop @stop-color=red}}}}{{linearGradient @id=b}}{{path @fill='url(#a) red'}}}}}}}}")}]);
    if clone {
        value["outputUri"] = BASE.into();
        value["sources"][0]["source"] = value["sources"][0]["source"]
            .as_str()
            .unwrap()
            .replace(
                "{linearGradient @id=a @href='#b' | {stop @stop-color=red}}{linearGradient @id=b}",
                "{linearGradient @id=a @href='#b'}{linearGradient @id=b | {stop @stop-color=red}}",
            )
            .into();
    }
    let mut s = RecordedOutputSession::from_json(&value.to_string(), 1, 1).unwrap();
    compose(&mut s);
    s.prepare_base(slots, |_| Ok(HtmlBasePolicyDecision::Allow))
        .unwrap();
    s.prepare_stylesheets(BASE, slots, None).unwrap();
    s
}
#[test]
fn paint_plan_is_published_only_with_validated_output_and_revoked_with_it() {
    let mut s = paint_session();
    assert_eq!(s.svg_paint_counts(), (0, 0));
    assert_eq!(s.svg_paint_content_counts(), (0, 0));
    assert_eq!(s.svg_paint_style_counts(), (0, 0));
    let blockers = s
        .capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert_eq!(s.svg_paint_counts(), (1, 1));
    assert_eq!(s.svg_paint_content_counts(), (1, 2));
    assert_eq!(s.svg_paint_style_counts(), (1, 0));
    assert!(blockers
        .iter()
        .any(|b| b == "svg-paint-clone-browser-compatibility-required"));
    assert!(blockers
        .iter()
        .any(|b| b == "svg-paint-clone-style-provenance-required"));
    assert!(blockers
        .iter()
        .any(|b| b == "svg-paint-base-context-required"));
    assert!(blockers
        .iter()
        .any(|b| b == "svg-paint-rendering-evidence-required"));
    assert_eq!(
        s.read(BASE, slots, &mut OutputObserver { wrong: false })
            .unwrap(),
        blockers
    );
    assert!(s
        .read(
            BASE,
            |_| Err("paint target changed".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert_eq!(s.svg_paint_counts(), (0, 0));
    assert_eq!(s.svg_paint_content_counts(), (0, 0));
    assert_eq!(s.svg_paint_style_counts(), (0, 0));
    assert!(s
        .read(BASE, slots, &mut OutputObserver { wrong: false })
        .is_err());
}
#[test]
fn failed_capture_and_close_do_not_publish_paint_candidates() {
    let mut s = paint_session();
    assert!(s
        .capture(
            BASE,
            |_| Err("target detached".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert_eq!(s.svg_paint_counts(), (0, 0));
    assert_eq!(s.svg_paint_content_counts(), (0, 0));
    assert_eq!(s.svg_paint_style_counts(), (0, 0));
    let mut s = paint_session();
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    s.close();
    assert_eq!(s.svg_paint_counts(), (0, 0));
    assert_eq!(s.svg_paint_content_counts(), (0, 0));
    assert_eq!(s.svg_paint_style_counts(), (0, 0));
}

#[test]
fn clone_contexts_publish_and_revoke_with_the_original_preparation() {
    let mut s = paint_session_with_clone(true);
    assert_eq!(s.svg_paint_style_counts(), (0, 0));
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert_eq!(s.svg_paint_style_counts(), (2, 1));
    s.read(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(s
        .read(
            BASE,
            |_| Err("clone source changed".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert_eq!(s.svg_paint_style_counts(), (0, 0));
    assert!(s
        .read(BASE, slots, &mut OutputObserver { wrong: false })
        .is_err());
}

fn clone_probe(
    case: cem_ml::svg_paint_clone_profile::CloneProbe,
) -> Result<cem_ml::svg_paint_clone_profile::CloneProbeObservation, String> {
    use cem_ml::svg_paint_clone_profile::*;
    let profile =
        SvgPaintCloneProfile::for_browser("chromium", CHROMIUM_CLONE_PROFILE_VERSION).unwrap();
    Ok(CloneProbeObservation {
        case,
        source_color: [255, 0, 0, 255],
        host_color: [0, 0, 255, 255],
        pixel: profile.expected_pixel(case),
    })
}
#[test]
fn explicit_clone_profile_confirmation_shares_preparation_lifetime() {
    let mut s = paint_session_with_clone(true);
    assert!(!s.clone_profile_confirmed());
    s.confirm_clone_profile("chromium-148-solid-currentcolor-v1", clone_probe)
        .unwrap();
    assert!(!s.clone_profile_confirmed()); // no publication before capture
    let blockers = s
        .capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(s.clone_profile_confirmed());
    assert!(blockers
        .iter()
        .any(|b| b == "svg-paint-clone-execution-evidence-required"));
    s.read(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(s
        .read(
            BASE,
            |_| Err("mutated".into()),
            &mut OutputObserver { wrong: false }
        )
        .is_err());
    assert!(!s.clone_profile_confirmed());
}
#[test]
fn clone_profile_rejects_unknown_repeated_mismatched_and_out_of_order_requests() {
    let name = "chromium-148-solid-currentcolor-v1";
    let mut s = paint_session_with_clone(true);
    assert!(s.confirm_clone_profile("unknown", clone_probe).is_err());
    assert!(s.html().is_err());
    let mut s = paint_session_with_clone(true);
    s.confirm_clone_profile(name, clone_probe).unwrap();
    assert!(s.confirm_clone_profile(name, clone_probe).is_err());
    assert!(s.html().is_err());
    let mut s = paint_session_with_clone(true);
    assert!(s
        .confirm_clone_profile(name, |case| {
            let mut value = clone_probe(case)?;
            value.pixel[0] ^= 1;
            Ok(value)
        })
        .is_err());
    assert!(s.html().is_err());
    let mut s = session();
    assert!(s.confirm_clone_profile(name, clone_probe).is_err());
    let mut s = paint_session_with_clone(true);
    s.capture(BASE, slots, &mut OutputObserver { wrong: false })
        .unwrap();
    assert!(!s.clone_profile_confirmed());
    assert!(s.confirm_clone_profile(name, clone_probe).is_err());
}
