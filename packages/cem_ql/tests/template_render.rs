use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};

fn string_value(value: &str) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::String(value.to_owned())))
}

fn bool_value(value: bool) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::Boolean(value)))
}

#[test]
fn render_template_binds_content_expression_from_host_data() {
    let data = TemplateData::default().with_binding("label", string_value("Email"));

    let rendered = render_template("{span | {$label}}", &data);

    assert_eq!(rendered.rendered, "<span>Email</span>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_interpolates_attribute_value_templates() {
    let data = TemplateData::default()
        .with_binding("tone", string_value("danger"))
        .with_binding("disabled", bool_value(true));

    let rendered = render_template(
        r#"{button @class="action {$tone}" @disabled="{$disabled}" | Save}"#,
        &data,
    );

    assert_eq!(
        rendered.rendered,
        r#"<button class="action danger" disabled="true">Save</button>"#
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_escapes_expression_output() {
    let data = TemplateData::default().with_binding("label", string_value("<Email & Phone>"));

    let rendered = render_template(r#"{span @title="{$label}" | {$label}}"#, &data);

    assert_eq!(
        rendered.rendered,
        r#"<span title="&lt;Email &amp; Phone&gt;">&lt;Email &amp; Phone&gt;</span>"#
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_reports_unknown_host_binding() {
    let rendered = render_template("{span | {$missing}}", &TemplateData::default());

    assert_eq!(rendered.rendered, "<span></span>");
    assert!(rendered
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));
}
