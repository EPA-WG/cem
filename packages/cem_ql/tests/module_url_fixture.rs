use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
    TemplateData,
};

#[test]
fn nested_source_resource_template_compiles_before_url_publication() {
    let story = include_str!("../../cem-elements/src/lib/cem-elements.stories.ts");
    let source = story
        .split("tag=\"story-nested-inline-resource\">")
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

    // Reuse the initially compiled artifact as the resource appears and clears.
    for url in [
        None,
        Some("https://fixtures.example.test/demo/asset.svg"),
        None,
    ] {
        let slices = Item::Record(
            url.into_iter()
                .map(|value| {
                    (
                        "asset".into(),
                        vec![Item::Atomic(AtomValue::String(value.into()))],
                    )
                })
                .collect(),
        );
        let data = TemplateData::default().with_binding(
            "datadom",
            ItemStream::once(Item::Record([("slices".into(), vec![slices])].into())),
        );
        let rendered = render_compiled_template(&artifact, &data);
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        let html = render_plan_to_html(&rendered);
        if let Some(url) = url {
            assert!(html.contains(&format!("href=\"{url}\"")), "{html}");
        } else {
            assert!(!html.contains(" href="), "{html}");
        }
        assert!(html.contains("Inline resource</a>"), "{html}");
    }
}
