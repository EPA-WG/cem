use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, HostAttributeUpdate, TemplateData};

#[test]
fn module_prelude_defaults_match_unwrapped_declarations() {
    let declarations = "{slice @name=scope | css-samples}{slice @name=enabled | false}{slice @name=empty}{attribute @name=label | Default}";
    let body = r#"{output | {$scope}|{$datadom.slices.scope}|{$ enabled == false}|{$ datadom.slices.enabled == false}|{$ empty ?? "unset"}|{$label}}"#;
    for source in [
        format!("{declarations}{body}"),
        format!("{declarations}{{body |{body}}}"),
        format!("{{module |{declarations}{body}}}"),
        format!("{{module |{declarations}{{body |{body}}}}}"),
        format!("{{cem:module |{declarations}{{cem:body |{body}}}}}"),
    ] {
        let result = render_template(&source, &TemplateData::default());
        assert!(
            result.diagnostics.is_empty(),
            "{source}: {:?}",
            result.diagnostics
        );
        assert_eq!(
            result.rendered,
            "<output>css-samples|css-samples|true|true|unset|Default</output>"
        );
        assert_eq!(
            result.host_attribute_updates,
            vec![HostAttributeUpdate::new("label", "Default")]
        );
    }
}

#[test]
fn module_defaults_preserve_host_empty_and_boolean_values() {
    let source = r#"{module |
        {slice @name=choice | default}{slice @name=enabled | true}
        {attribute @name=label | Default}
        {body | {output | {$ str:length(choice)}|{$ enabled == false}|{$ str:length(label)}}}
    }"#;
    let data = TemplateData::default()
        .with_binding(
            "choice",
            ItemStream::once(Item::Atomic(AtomValue::String("".into()))),
        )
        .with_binding(
            "enabled",
            ItemStream::once(Item::Atomic(AtomValue::Boolean(false))),
        )
        .with_binding(
            "label",
            ItemStream::once(Item::Atomic(AtomValue::String("".into()))),
        );
    let result = render_template(source, &data);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.rendered, "<output>0|true|0</output>");
    assert!(result.host_attribute_updates.is_empty());
}

#[test]
fn defaults_do_not_escape_named_templates_or_body_content() {
    let result = render_template(
        r#"{module |
        {template @name=unused | {slice @name=private | hidden}{output | unused}}
        {body | {article | {slice @name=nested | hidden}}
            {output | {$ datadom.slices.private ?? "unset"}|{$ datadom.slices.nested ?? "unset"}}}
    }"#,
        &TemplateData::default(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(
        result.rendered.contains("<output>unset|unset</output>"),
        "{}",
        result.rendered
    );
}

#[test]
fn authored_dynamic_style_sample_has_only_its_two_intended_diagnostics() {
    let page = include_str!("../../cem-elements/demo/scoped-css.html");
    let source = page
        .split_once("<cem-element tag=\"cem-css-dynamic\">")
        .unwrap()
        .1
        .split_once("<template type=\"text/cem-ml\">")
        .unwrap()
        .1
        .split_once("</template>")
        .unwrap()
        .0;
    let result = render_template(source, &TemplateData::default());
    assert_eq!(result.diagnostics.len(), 2, "{:?}", result.diagnostics);
    assert!(result
        .diagnostics
        .iter()
        .all(|d| d.code == "cem.ql.template.stylesheet_dynamic_unsupported"));
    assert!(!result.rendered.contains("<style"));
    assert!(result.rendered.contains("dynamic styles rejected"));
    assert!(result.diagnostics.iter().all(|d| d.source_map.is_some()));
}
