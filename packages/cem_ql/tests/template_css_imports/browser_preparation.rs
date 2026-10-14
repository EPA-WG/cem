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
