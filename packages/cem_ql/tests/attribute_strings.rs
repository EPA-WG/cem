use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
    HostAttributeUpdate, TemplateData,
};

#[test]
fn attribute_demo_distinguishes_missing_empty_strings_and_typed_booleans() {
    let demo = include_str!("../../cem-elements/demo/attributes.html");
    let source = demo
        .split("<cem-element tag=\"cem-attr-defaults\">")
        .nth(1)
        .unwrap()
        .split("<template type=\"text/cem-ml\">")
        .nth(1)
        .unwrap()
        .split("</template>")
        .next()
        .unwrap();
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["datadom".into()],
            ..CompileTemplateOptions::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );

    for (value, expected) in [
        (None, "def_P3"),
        (Some(AtomValue::String("".into())), ""),
        (Some(AtomValue::String("false".into())), "false"),
        (Some(AtomValue::Boolean(true)), "true"),
        (Some(AtomValue::Boolean(false)), "false"),
    ] {
        let attributes = Item::Record(
            value
                .into_iter()
                .map(|value| ("p3".into(), vec![Item::Atomic(value)]))
                .collect(),
        );
        let data = TemplateData::default().with_binding(
            "datadom",
            ItemStream::once(Item::Record(
                [("attributes".into(), vec![attributes])].into(),
            )),
        );
        let rendered = render_compiled_template(&artifact, &data);
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        assert!(
            rendered
                .host_attribute_updates
                .contains(&HostAttributeUpdate::new("p3", expected)),
            "{:?}",
            rendered.host_attribute_updates,
        );
        let html = render_plan_to_html(&rendered);
        assert!(html.contains(&format!("<p>p3: {expected}</p>")), "{html}");
    }
}

#[test]
fn canonical_boolean_attribute_projects_presence_without_coercing_strings() {
    let stories = include_str!("../../cem-elements/src/lib/cem-elements.stories.ts");
    let source = stories
        .split("export const CanonicalCemMlRenderLoop")
        .nth(1)
        .unwrap()
        .split("template.textContent = `")
        .nth(1)
        .unwrap()
        .split('`')
        .next()
        .unwrap();
    let artifact = compile_template(source, &CompileTemplateOptions::default());
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for value in [None, Some(""), Some("false"), Some("busy")] {
        let attributes = Item::Record(
            value
                .into_iter()
                .map(|value| {
                    (
                        "busy".into(),
                        vec![Item::Atomic(AtomValue::String(value.into()))],
                    )
                })
                .collect(),
        );
        let data = TemplateData::default().with_binding(
            "datadom",
            ItemStream::once(Item::Record(
                [("attributes".into(), vec![attributes])].into(),
            )),
        );
        let rendered = render_compiled_template(&artifact, &data);
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        let html = render_plan_to_html(&rendered);
        assert_eq!(
            html.contains("aria-busy=\"true\""),
            value.is_some(),
            "{html}"
        );
        assert!(html.trim().ends_with(">Save</button>"), "{html}");
    }
}

#[test]
fn select_boolean_attributes_use_presence_with_exact_dom_strings() {
    let declaration =
        include_str!("../../cem-components/src/components/cem-select/cem-select.xhtml");
    let source = declaration
        .split("<template id=\"cem-select\" type=\"text/cem-ml\">")
        .nth(1)
        .unwrap()
        .split("</template>")
        .next()
        .unwrap();
    let artifact = compile_template(source, &CompileTemplateOptions::default());
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    for mode in ["dropdown", "listbox"] {
        for value in [None, Some(""), Some("false"), Some("present")] {
            let attributes = Item::Record(
                ["busy", "invalid", "disabled"]
                    .into_iter()
                    .filter_map(|name| {
                        value.map(|value| {
                            (
                                name.into(),
                                vec![Item::Atomic(AtomValue::String(value.into()))],
                            )
                        })
                    })
                    .collect(),
            );
            let mode_value = Item::Atomic(AtomValue::String(mode.into()));
            let slices = Item::Record(
                [
                    ("mode".into(), vec![mode_value.clone()]),
                    (
                        "behaviorDisabled".into(),
                        vec![Item::Atomic(AtomValue::Boolean(false))],
                    ),
                ]
                .into(),
            );
            let data = TemplateData::default()
                .with_binding(
                    "label",
                    ItemStream::once(Item::Atomic(AtomValue::String("Role".into()))),
                )
                .with_binding("mode", ItemStream::once(mode_value))
                .with_binding(
                    "datadom",
                    ItemStream::once(Item::Record(
                        [
                            ("attributes".into(), vec![attributes]),
                            ("slices".into(), vec![slices]),
                        ]
                        .into(),
                    )),
                );
            let rendered = render_compiled_template(&artifact, &data);
            assert!(
                rendered.diagnostics.is_empty(),
                "{:?}",
                rendered.diagnostics
            );
            let html = render_plan_to_html(&rendered);
            for attribute in [
                "data-state=\"loading\"",
                "aria-busy=\"true\"",
                "aria-invalid=\"true\"",
            ] {
                assert_eq!(
                    html.contains(attribute),
                    value.is_some(),
                    "{mode} {value:?}: {attribute}\n{html}"
                );
            }
            if mode == "dropdown" {
                assert_eq!(
                    html.contains(" disabled=\"true\""),
                    value.is_some(),
                    "{html}"
                );
            } else {
                let tabindex = if value.is_some() { -1 } else { 0 };
                assert!(html.contains(&format!("tabindex=\"{tabindex}\"")), "{html}");
                assert_eq!(
                    html.contains("aria-disabled=\"true\""),
                    value.is_some(),
                    "{html}"
                );
            }
        }
    }
}
