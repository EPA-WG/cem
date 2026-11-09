use cem_ml::{import::dom::DomCapture, operation_control::OperationControl};
use cem_ql::{
    eval::RetainedCemNode,
    ssr_browser_resume::{admit, ResumeDecision},
};
fn host(version: Option<&str>, text: &str) -> cem_ql::eval::Item {
    host_case(version, text, "")
}
fn host_case(version: Option<&str>, text: &str, case: &str) -> cem_ql::eval::Item {
    let mut c = DomCapture::new(
        "cem-dom://runtime/resume",
        "https://example.test/",
        "https://example.test/",
        Default::default(),
    )
    .unwrap();
    let mut seq = 0;
    let mut push = |parent, kind: &str, ns: &str, name: &str, value: &str| {
        seq += 1;
        c.push(parent, &format!("n-{seq}"), kind, ns, name, value)
            .unwrap()
    };
    push(0, "document", "", "", "");
    let h = push_host(&mut push, 0, "x-card", "ssr-1", version, text, case);
    let tree = c.finish_literal_namespaces().unwrap();
    RetainedCemNode::new(tree, h).unwrap().query_item()
}

fn push_host(
    push: &mut impl FnMut(u32, &str, &str, &str, &str) -> u32,
    parent: u32,
    tag: &str,
    instance_id: &str,
    version: Option<&str>,
    text: &str,
    case: &str,
) -> u32 {
    let h = push(parent, "element", "http://www.w3.org/1999/xhtml", tag, "");
    if case == "host-input" {
        push(h, "attribute", "", "title", "Changed");
    }
    let t = push(h, "element", "http://www.w3.org/1999/xhtml", "template", "");
    push(t, "attribute", "", "data-cem-island", "instance");
    push(
        t,
        "attribute",
        "",
        cem_ql::ssr_browser_resume::PROFILE_ATTRIBUTE,
        if case == "profile" {
            "unknown/2"
        } else {
            cem_ql::ssr_browser_resume::PROFILE
        },
    );
    let r = push(
        t,
        "element",
        "http://www.w3.org/1999/xhtml",
        "cem-island:context-root",
        "",
    );
    let island_version = if case == "future" { "0.3.0" } else { "0.2.0" };
    push(r, "attribute", "", "version", island_version);
    for (prefix, ns) in [
        ("cem-island", "data-island"),
        ("cem-hydration", "hydration-data"),
        ("cem-attributes", "instance-attributes"),
        ("cem-dataset", "instance-dataset"),
        ("cem-payload", "instance-payload"),
        ("cem-slices", "instance-slices"),
        ("cem-resources", "resource-state"),
        ("cem-form", "form-state"),
        ("cem-validation", "validation-state"),
        ("cem-events", "event-state"),
    ] {
        push(
            r,
            "attribute",
            "",
            &format!("xmlns:{prefix}"),
            &format!("https://cem.dev/ns/runtime/{ns}"),
        );
    }
    for (i, (local, ns)) in cem_ql::instance_island::PARTS.iter().enumerate() {
        let prefix = [
            "cem-hydration",
            "cem-attributes",
            "cem-dataset",
            "cem-payload",
            "cem-slices",
            "cem-resources",
            "cem-form",
            "cem-validation",
            "cem-events",
        ][i];
        let p = push(r, "element", ns, &format!("{prefix}:{local}"), "");
        if i == 0 {
            let mut fields = vec![
                ("version", island_version),
                ("instanceId", instance_id),
                ("outputTarget", "light-dom"),
            ];
            if let Some(v) = version {
                fields.push(("declarationVersion", v));
            }
            if case == "duplicate" {
                fields.push(("instanceId", "other"));
            }
            if case == "unknown" {
                fields.push(("resourceTicket", "forged"));
            }
            for (name, value) in fields {
                let f = push(p, "element", ns, "cem-hydration:field", "");
                push(f, "attribute", "", "name", name);
                push(f, "attribute", "", "kind", "string");
                push(f, "text", "", "", value);
            }
        }
        if i == 3 {
            push(p, "attribute", "", "content-type", "text/html");
            push(
                p,
                "attribute",
                "",
                "schema",
                cem_ml::schema::registry::HTML_SCHEMA_URI,
            );
            let b = push(
                p,
                "element",
                "http://www.w3.org/1999/xhtml",
                if case == "active" { "script" } else { "em" },
                "",
            );
            if case == "slot" {
                push(b, "attribute", "", "slot", "heading");
            }
            push(b, "text", "", "", "Input");
        }
    }
    push(h, "comment", "", "", "cem-render-start");
    let b = push(h, "element", "http://www.w3.org/1999/xhtml", "b", "");
    if case.starts_with("nested") {
        push_host(
            push,
            b,
            "x-child",
            if case == "nested-duplicate" {
                instance_id
            } else {
                "ssr-2"
            },
            Some(if case == "nested-version" {
                "2.0.0"
            } else {
                "1.2.3"
            }),
            text,
            "",
        );
    } else if case == "slot" {
        let e = push(b, "element", "http://www.w3.org/1999/xhtml", "em", "");
        push(e, "attribute", "", "slot", "heading");
        push(e, "text", "", "", text);
    } else {
        push(b, "text", "", "", text);
    }
    push(h, "comment", "", "", "cem-render-end");
    if case == "range" {
        push(h, "comment", "", "", "cem-render-start");
    }
    h
}

#[test]
fn nested_group_requires_fresh_output_and_parent_owned_inputs_for_every_child() {
    use cem_ql::ssr_browser_resume::{admit_group, NestedDeclaration};

    let control = OperationControl::default();
    let declaration = NestedDeclaration {
        tag: "x-child".into(),
        version: "1.9.0".into(),
        source: "{b | Nested}".into(),
    };
    let run = |case, text, source| {
        admit_group(
            &host_case(Some("1.2.3"), text, case),
            "x-card",
            "1.9.0",
            source,
            std::slice::from_ref(&declaration),
            &control,
        )
    };
    let source = "{b | {x-child | {em | Input}}}";
    let admitted = run("nested", "Nested", source).unwrap();
    assert_eq!(
        admitted
            .participants
            .iter()
            .map(|p| p.instance_id.as_str())
            .collect::<Vec<_>>(),
        ["ssr-1", "ssr-2"]
    );
    for (case, text, source) in [
        ("nested", "Tampered", source),
        ("nested", "Nested", "{b | {x-child | {em | Wrong input}}}"),
        (
            "nested",
            "Nested",
            "{b | {x-child @title=Wrong | {em | Input}}}",
        ),
        ("nested", "Nested", "{section | {x-child | {em | Input}}}"),
        ("nested-duplicate", "Nested", source),
        ("nested-version", "Nested", source),
    ] {
        assert!(run(case, text, source).is_err(), "{case} {source}");
    }
}
#[test]
fn compatible_output_requires_fresh_native_render_agreement() {
    let c = OperationControl::default();
    let h = host(Some("1.2.3"), "Ready");
    let ready = admit(&h, "x-card", "1.9.0", "{b | Ready}", &c).unwrap();
    assert_eq!(ready.decision, ResumeDecision::Adopt);
    assert_eq!(ready.instance_id, "ssr-1");
    assert!(admit(
        &host(Some("1.2.3"), "Tampered"),
        "x-card",
        "1.9.0",
        "{b | Ready}",
        &c
    )
    .is_err());
    c.abort_signal().abort();
    assert!(admit(&h, "x-card", "1.9.0", "{b | Ready}", &c).is_err());
}
#[test]
fn missing_malformed_and_incompatible_versions_only_allow_payload_render_on_load() {
    for version in [None, Some("v1"), Some("2.0.0"), Some("1.2.3-beta.1")] {
        let ready = admit(
            &host(version, "Stale output"),
            "x-card",
            "1.9.0",
            "{b | Ready}",
            &OperationControl::default(),
        )
        .unwrap();
        assert_eq!(ready.decision, ResumeDecision::RenderOnLoad);
    }
}
#[test]
fn wrong_host_or_unsupported_styles_never_adopt() {
    let h = host(Some("1.2.3"), "Ready");
    let c = OperationControl::default();
    assert!(admit(&h, "x-other", "1.9.0", "{b | Ready}", &c).is_err());
    assert!(admit(
        &h,
        "x-card",
        "1.9.0",
        "{style |```b {color:red}```}{b | Ready}",
        &c
    )
    .is_err());
}

#[test]
fn native_capture_crosses_only_the_typed_value_boundary() {
    use cem_ql::eval::{
        portable::{decode_values, encode_values},
        ItemStream,
    };
    let limits = Default::default();
    let bytes = encode_values(
        &ItemStream::once(host(Some("0.2.3+build"), "Ready")),
        &limits,
    )
    .unwrap();
    let values = decode_values(&bytes, &limits).unwrap();
    let c = OperationControl::default();
    assert_eq!(
        admit(&values.items[0], "x-card", "0.2.9", "{b | Ready}", &c)
            .unwrap()
            .decision,
        ResumeDecision::Adopt
    );
    assert_eq!(
        admit(&values.items[0], "x-card", "0.3.0", "{b | Ready}", &c)
            .unwrap()
            .decision,
        ResumeDecision::RenderOnLoad
    );
}

#[test]
fn invalid_islands_ranges_and_inputs_freeze_even_when_versions_differ() {
    for case in [
        "future",
        "profile",
        "duplicate",
        "unknown",
        "active",
        "host-input",
        "range",
    ] {
        assert!(
            admit(
                &host_case(Some("2.0.0"), "Ready", case),
                "x-card",
                "1.2.3",
                "{b | Ready}",
                &OperationControl::default()
            )
            .is_err(),
            "{case}"
        );
    }
}

#[test]
fn current_native_plan_uses_admitted_slot_payload_instead_of_output_text() {
    let c = OperationControl::default();
    let source = "{b | {$ datadom.slots.heading}}";
    assert_eq!(
        admit(
            &host_case(Some("1.2.3"), "Input", "slot"),
            "x-card",
            "1.9.0",
            source,
            &c
        )
        .unwrap()
        .decision,
        ResumeDecision::Adopt
    );
    assert!(admit(
        &host_case(Some("1.2.3"), "Changed", "slot"),
        "x-card",
        "1.9.0",
        source,
        &c
    )
    .is_err());
}
