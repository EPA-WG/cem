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
        if i == 4 && case.starts_with("state-") {
            let slice = push(p, "element", ns, "cem-slices:slice", "");
            push(
                slice,
                "attribute",
                "",
                "name",
                if case == "state-undeclared" {
                    "other"
                } else {
                    "message"
                },
            );
            push(slice, "attribute", "", "kind", "string");
            push(
                slice,
                "text",
                "",
                "",
                if case == "state-tampered" {
                    "Changed"
                } else {
                    "Ready"
                },
            );
        }
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
            if case.starts_with("source-") {
                push(p, "attribute", "", "content-type", "text/cem-ml");
                push(
                    p,
                    "attribute",
                    "",
                    "schema",
                    cem_ml::schema::registry::CEM_ML_SCHEMA_URI,
                );
                push(p, "attribute", "", "data-cem-explicit-payload", "true");
                let envelope = push(p, "element", "http://www.w3.org/1999/xhtml", "template", "");
                push(envelope, "attribute", "", "type", "text/cem-ml");
                if case == "source-markup" {
                    push(envelope, "element", "http://www.w3.org/1999/xhtml", "p", "");
                } else {
                    push(
                        envelope,
                        "text",
                        "",
                        "",
                        if case == "source-malformed" {
                            "{unclosed |"
                        } else {
                            "{item | {$ 1 div 0}}"
                        },
                    );
                }
                continue;
            }
            push(p, "attribute", "", "content-type", "text/html");
            push(
                p,
                "attribute",
                "",
                "schema",
                cem_ml::schema::registry::HTML_SCHEMA_URI,
            );
            if case.starts_with("foreign") {
                push_foreign(push, p, false, false);
                continue;
            }
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
    if case.starts_with("state-") {
        let html = "http://www.w3.org/1999/xhtml";
        let form = push(b, "element", html, "form", "");
        if case == "state-route" {
            push(form, "attribute", "", "action", "#other");
        }
        let input = push(form, "element", html, "input", "");
        push(input, "attribute", "", "name", "message");
        push(input, "attribute", "", "slice", "message");
        if case == "state-private" {
            push(
                input,
                "attribute",
                "",
                "autocomplete",
                "section-account one-time-code",
            );
        }
    } else if case.starts_with("foreign") {
        push_foreign(push, b, case == "foreign-range", case == "foreign-active");
    } else if case.starts_with("nested") {
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
            if case == "nested-source" {
                "source-valid"
            } else {
                ""
            },
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
fn browser_resume_proves_slice_defaults_and_form_policy() {
    let run = |case, form_attrs, input_attrs, version| {
        let source = format!("{{slice @name=message | Ready}}{{b | {{form {form_attrs} | {{input @name=message @slice=message {input_attrs}}}}}}}");
        admit(
            &host_case(Some("1.2.3"), "", case),
            "x-card",
            version,
            &source,
            &OperationControl::default(),
        )
    };
    assert_eq!(
        run("state-valid", "", "", "1.9.0").unwrap().decision,
        ResumeDecision::Adopt
    );
    assert!(run("state-valid", "", "", "2.0.0").is_err());
    for case in ["state-tampered", "state-undeclared"] {
        assert!(run(case, "", "", "1.9.0").is_err(), "{case}");
    }
    assert!(run("state-route", "@action='#other'", "", "1.9.0").is_err());
    assert!(run(
        "state-private",
        "",
        "@autocomplete='section-account one-time-code'",
        "1.9.0"
    )
    .is_err());
}

fn push_foreign(
    push: &mut impl FnMut(u32, &str, &str, &str, &str) -> u32,
    parent: u32,
    changed: bool,
    active: bool,
) {
    const SVG: &str = "http://www.w3.org/2000/svg";
    const MATH: &str = "http://www.w3.org/1998/Math/MathML";
    let svg = push(parent, "element", SVG, "svg", "");
    push(svg, "attribute", "", "slot", "heading");
    push(svg, "attribute", "", "viewBox", "0 0 24 24");
    push(
        svg,
        "attribute",
        "http://www.w3.org/XML/1998/namespace",
        "lang",
        "en",
    );
    let gradient = push(svg, "element", SVG, "linearGradient", "");
    push(gradient, "attribute", "", "id", "paint");
    let use_node = push(svg, "element", SVG, "use", "");
    push(
        use_node,
        "attribute",
        "http://www.w3.org/1999/xlink",
        "href",
        if active {
            "https://external.test/a.svg"
        } else if changed {
            "#changed"
        } else {
            "#paint"
        },
    );
    let math = push(parent, "element", MATH, "math", "");
    push(math, "attribute", "", "slot", "heading");
    let identifier = push(math, "element", MATH, "mi", "");
    push(identifier, "text", "", "", "x");
}

#[test]
fn passive_foreign_ranges_compare_expanded_names_and_namespaced_attributes() {
    let control = OperationControl::default();
    let resume = |case| {
        admit(
            &host_case(Some("1.2.3"), "", case),
            "x-card",
            "1.9.0",
            "{b | {$ datadom.slots.heading}}",
            &control,
        )
    };
    assert_eq!(resume("foreign").unwrap().decision, ResumeDecision::Adopt);
    assert!(resume("foreign-range").is_err());
    assert!(resume("foreign-active").is_err());
}

#[test]
fn cem_source_resume_requires_fresh_capture_and_keeps_expressions_inert() {
    use cem_ql::eval::{
        portable::{decode_values, encode_values},
        ItemStream,
    };

    let control = OperationControl::default();
    let resume = |host: &_| admit(host, "x-card", "1.9.0", "{b | Ready}", &control);
    let host = host_case(Some("1.2.3"), "Ready", "source-valid");
    assert_eq!(resume(&host).unwrap().decision, ResumeDecision::Adopt);
    assert!(admit(&host, "x-card", "2.0.0", "{b | Ready}", &control).is_err());
    let limits = Default::default();
    let bytes = encode_values(&ItemStream::once(host), &limits).unwrap();
    let restored = decode_values(&bytes, &limits).unwrap();
    assert!(resume(&restored.items[0]).is_err());
    for case in ["source-malformed", "source-markup"] {
        assert!(
            resume(&host_case(Some("1.2.3"), "Ready", case)).is_err(),
            "{case}"
        );
    }
}

#[test]
fn captured_cem_source_keeps_the_dom_owner_and_unknown_coordinates() {
    use cem_ml::{
        import::element_tree::retain_captured_cem_payload,
        source_map::{FrameSpan, TransformKind},
        value::artifact::CemValueArtifactLimits,
    };
    let uri = "cem-dom://runtime/payload";
    let mut capture = DomCapture::new(uri, "", "", Default::default()).unwrap();
    capture.push(0, "document", "document", "", "", "").unwrap();
    let envelope = capture
        .push(
            0,
            "template",
            "element",
            "http://www.w3.org/1999/xhtml",
            "template",
            "",
        )
        .unwrap();
    capture
        .push(envelope, "type", "attribute", "", "type", "text/cem-ml")
        .unwrap();
    capture
        .push(envelope, "body", "text", "", "", "{item | Captured}")
        .unwrap();
    let tree = capture.finish().unwrap();
    let control = OperationControl::default();
    let source =
        retain_captured_cem_payload(tree.clone(), envelope, &control, &Default::default()).unwrap();
    assert!(source.matches_canonical_source(&tree, envelope));
    assert!(source.matches_operation(&control));
    let derived = source.tree();
    let item = derived.node(0).unwrap().children[0];
    let map = derived.node_source_map(item).unwrap();
    assert!(map.frames.iter().any(|frame| {
        matches!(&frame.transform, TransformKind::SourceBoundary { uri: source } if source == uri)
            && matches!(&frame.span, FrameSpan::Multi(spans) if spans.is_empty())
    }));
    assert!(retain_captured_cem_payload(
        tree,
        envelope,
        &control,
        &CemValueArtifactLimits {
            max_bytes: 4,
            ..Default::default()
        },
    )
    .is_err());
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
    let source = "{b | {x-child | {template @type=cem-ml | ```{item | {$ 1 div 0}}```}}}";
    assert_eq!(
        run("nested-source", "Nested", source)
            .unwrap()
            .participants
            .len(),
        2
    );
    let wrong_source = source.replace("div 0", "div 1");
    assert!(run("nested-source", "Nested", &wrong_source).is_err());
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
