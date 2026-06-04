use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, CompileTemplateOptions, RenderPlanNode,
    TemplateData,
};
use cem_ql::render::{render_plan_to_html, render_template};

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

#[test]
fn compiled_template_renders_multiple_snapshots_without_recompile() {
    let artifact = compile_template(
        "{span | {$label}}",
        &CompileTemplateOptions {
            host_bindings: vec!["label".to_owned()],
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );

    let first = render_compiled_template(
        &artifact,
        &TemplateData::default().with_binding("label", string_value("Email")),
    );
    let second = render_compiled_template(
        &artifact,
        &TemplateData::default().with_binding("label", string_value("Phone")),
    );

    assert_eq!(render_plan_to_html(&first), "<span>Email</span>");
    assert_eq!(render_plan_to_html(&second), "<span>Phone</span>");
}

#[test]
fn render_plan_preserves_structured_nodes_and_source_maps() {
    let artifact = compile_template(
        r#"{button @class="action {$tone}" | {$label}}"#,
        &CompileTemplateOptions {
            host_bindings: vec!["tone".to_owned(), "label".to_owned()],
        },
    );
    let plan = render_compiled_template(
        &artifact,
        &TemplateData::default()
            .with_binding("tone", string_value("primary"))
            .with_binding("label", string_value("Save")),
    );

    let [RenderPlanNode::Element {
        tag,
        attributes,
        children,
        source_map,
    }] = plan.nodes.as_slice()
    else {
        panic!("expected one rendered element");
    };

    assert_eq!(tag, "button");
    assert_eq!(attributes[0].name, "class");
    assert_eq!(attributes[0].value, "action primary");
    assert_eq!(children.len(), 1);
    assert!(!source_map.frames.is_empty(), "element carries source map");
}
