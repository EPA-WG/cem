use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, CompileTemplateOptions, RenderPlanNode,
    TemplateData,
};
use cem_ql::render::{render_plan_to_html, render_template};
use std::collections::BTreeMap;

fn string_value(value: &str) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::String(value.to_owned())))
}

fn bool_value(value: bool) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::Boolean(value)))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Vec<Item>)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
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

// --- C2.4: functional data-document selection (no XPath engine) + `??` --------

#[test]
fn render_template_selects_from_data_document() {
    let data = TemplateData::default().with_binding("label", string_value("Email"));

    // Functional parity with the legacy `/datadom/attributes/label` XPath selection,
    // expressed through cem-ql record navigation.
    let rendered = render_template("{span | {$datadom.attributes.label}}", &data);

    assert_eq!(rendered.rendered, "<span>Email</span>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_uses_explicit_structured_data_document() {
    let datadom = record([
        (
            "attributes",
            vec![record([(
                "label",
                vec![Item::Atomic(AtomValue::String("Email".to_owned()))],
            )])],
        ),
        (
            "dataset",
            vec![record([(
                "variant",
                vec![Item::Atomic(AtomValue::String("compact".to_owned()))],
            )])],
        ),
        (
            "slices",
            vec![record([(
                "open",
                vec![Item::Atomic(AtomValue::Boolean(true))],
            )])],
        ),
        (
            "payload",
            vec![record([(
                "text",
                vec![Item::Atomic(AtomValue::String("Payload".to_owned()))],
            )])],
        ),
        (
            "slots",
            vec![record([(
                "leading",
                vec![Item::Array(vec![record([(
                    "text",
                    vec![Item::Atomic(AtomValue::String("Lead".to_owned()))],
                )])])],
            )])],
        ),
    ]);
    let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

    let rendered = render_template(
        "{span | {$datadom.attributes.label}-{$datadom.dataset.variant}-{$datadom.slices.open}-{$datadom.payload.text}-{$datadom.slots.leading}}",
        &data,
    );

    assert_eq!(
        rendered.rendered,
        "<span>Email-compact-true-Payload-</span>"
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_coalesces_absent_selection_to_default() {
    let rendered = render_template(
        r#"{span | {$datadom.attributes.missing ?? "fallback"}}"#,
        &TemplateData::default(),
    );

    assert_eq!(rendered.rendered, "<span>fallback</span>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_coalesce_prefers_present_selection() {
    let data = TemplateData::default().with_binding("label", string_value("Email"));

    let rendered = render_template(
        r#"{span | {$datadom.attributes.label ?? "fallback"}}"#,
        &data,
    );

    assert_eq!(rendered.rendered, "<span>Email</span>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_coalesces_chained_selections() {
    let data = TemplateData::default().with_binding("alt", string_value("Alt"));

    // `a ?? b ?? c`: first present wins left-to-right.
    let rendered = render_template(
        r#"{span | {$datadom.attributes.missing ?? datadom.attributes.alt ?? "fallback"}}"#,
        &data,
    );

    assert_eq!(rendered.rendered, "<span>Alt</span>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

// --- C2.5: conditional constructs (cem:if / cem:choose / cem:when / cem:otherwise) ---

#[test]
fn render_template_if_emits_body_only_when_test_is_truthy() {
    let template = r#"{cem:if @test="show" | {span | yes}}"#;

    let shown = render_template(
        template,
        &TemplateData::default().with_binding("show", bool_value(true)),
    );
    assert_eq!(shown.rendered, "<span>yes</span>");
    assert!(shown.diagnostics.is_empty(), "{:?}", shown.diagnostics);

    let hidden = render_template(
        template,
        &TemplateData::default().with_binding("show", bool_value(false)),
    );
    assert_eq!(hidden.rendered, "");
    assert!(hidden.diagnostics.is_empty(), "{:?}", hidden.diagnostics);
}

#[test]
fn render_template_choose_selects_first_truthy_branch_else_otherwise() {
    let template = concat!(
        r#"{cem:choose | "#,
        r#"{cem:when @test="a" | {b | A}}"#,
        r#"{cem:when @test="c" | {b | C}}"#,
        r#"{cem:otherwise | {b | none}}}"#
    );

    let pick_c = render_template(
        template,
        &TemplateData::default()
            .with_binding("a", bool_value(false))
            .with_binding("c", bool_value(true)),
    );
    assert_eq!(pick_c.rendered, "<b>C</b>");

    let pick_a = render_template(
        template,
        &TemplateData::default()
            .with_binding("a", bool_value(true))
            .with_binding("c", bool_value(true)),
    );
    assert_eq!(pick_a.rendered, "<b>A</b>", "first truthy branch wins");

    let pick_otherwise = render_template(
        template,
        &TemplateData::default()
            .with_binding("a", bool_value(false))
            .with_binding("c", bool_value(false)),
    );
    assert_eq!(pick_otherwise.rendered, "<b>none</b>");
}

#[test]
fn render_template_accepts_bare_conditional_names() {
    let shown = render_template(
        r#"{if @test="show" | {span | yes}}"#,
        &TemplateData::default().with_binding("show", bool_value(true)),
    );
    assert_eq!(shown.rendered, "<span>yes</span>");
}

#[test]
fn render_template_if_tests_data_document_selection() {
    let shown = render_template(
        r#"{cem:if @test="datadom.attributes.label" | {span | {$datadom.attributes.label}}}"#,
        &TemplateData::default().with_binding("label", string_value("Email")),
    );
    assert_eq!(shown.rendered, "<span>Email</span>");

    let hidden = render_template(
        r#"{cem:if @test="datadom.attributes.label" | {span | x}}"#,
        &TemplateData::default(),
    );
    assert_eq!(hidden.rendered, "");
    assert!(hidden.diagnostics.is_empty(), "{:?}", hidden.diagnostics);
}

#[test]
fn render_template_reports_missing_conditional_tests() {
    let rendered = render_template(
        r#"{cem:choose | {cem:when | {span | yes}}}{cem:if | {span | no}}"#,
        &TemplateData::default(),
    );

    let missing_test_count = rendered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "cem.ql.render.conditional_test_missing")
        .count();
    assert_eq!(missing_test_count, 2, "{:?}", rendered.diagnostics);
}

#[test]
fn render_template_reports_invalid_choose_structure() {
    let rendered = render_template(
        r#"{cem:choose | {span | stray}{cem:otherwise @test="false" | {b | first}}{cem:otherwise | {b | second}}}"#,
        &TemplateData::default(),
    );

    assert_eq!(rendered.rendered, "<b>first</b>");
    assert!(rendered
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.render.choose_invalid_child"));
    assert!(rendered
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.render.otherwise_test_not_allowed"));
    assert!(rendered
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.render.choose_multiple_otherwise"));
}

#[test]
fn render_template_drops_top_level_attribute_and_slice_declarations() {
    // `<attribute>`/`<slice>` declarations configure the produced element and must not
    // appear in render output; only the `<button>` (with its resolved `$label`) renders.
    let data = TemplateData::default().with_binding("label", string_value("Save"));
    let rendered = render_template(
        r#"{attribute @name="label" | Save}{slice @name="open"}{button @type=button | {$label}}"#,
        &data,
    );

    assert_eq!(rendered.rendered, r#"<button type="button">Save</button>"#);
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

#[test]
fn render_template_applies_declaration_defaults() {
    let template = r#"{attribute @name="label" | Save}{button @type=button | {$label}}"#;

    // No host data → the declared default seeds `$label` (the render engine owns defaults).
    let default_render = render_template(template, &TemplateData::default());
    assert_eq!(default_render.rendered, r#"<button type="button">Save</button>"#);
    assert!(default_render.diagnostics.is_empty(), "{:?}", default_render.diagnostics);

    // A host-provided value overrides the declared default.
    let override_render = render_template(
        template,
        &TemplateData::default().with_binding("label", string_value("Submit")),
    );
    assert_eq!(override_render.rendered, r#"<button type="button">Submit</button>"#);
    assert!(override_render.diagnostics.is_empty(), "{:?}", override_render.diagnostics);
}

#[test]
fn render_template_supports_nested_conditionals() {
    // `cem:if` wrapping a `cem:choose` whose `cem:otherwise` nests another `cem:if`.
    let template = concat!(
        r#"{cem:if @test="outer" | "#,
        r#"{cem:choose | "#,
        r#"{cem:when @test="a" | {span | A}}"#,
        r#"{cem:otherwise | {cem:if @test="b" | {span | B}}}}}"#,
    );

    // outer true, when `a` false, nested `b` true -> the nested if in `otherwise` emits B.
    let nested_b = render_template(
        template,
        &TemplateData::default()
            .with_binding("outer", bool_value(true))
            .with_binding("a", bool_value(false))
            .with_binding("b", bool_value(true)),
    );
    assert_eq!(nested_b.rendered, "<span>B</span>");
    assert!(nested_b.diagnostics.is_empty(), "{:?}", nested_b.diagnostics);

    // outer false -> the whole subtree is skipped regardless of inner tests.
    let skipped = render_template(
        template,
        &TemplateData::default()
            .with_binding("outer", bool_value(false))
            .with_binding("a", bool_value(true))
            .with_binding("b", bool_value(true)),
    );
    assert_eq!(skipped.rendered, "");

    // outer true, when `a` true -> the matching `when` wins; `otherwise`/nested-if is not taken.
    let when_a = render_template(
        template,
        &TemplateData::default()
            .with_binding("outer", bool_value(true))
            .with_binding("a", bool_value(true))
            .with_binding("b", bool_value(false)),
    );
    assert_eq!(when_a.rendered, "<span>A</span>");
}

// --- cem:for-each iteration (the CSS-generator conversion prerequisite) ---

#[test]
fn render_template_for_each_iterates_a_sequence() {
    // A multi-item host binding; for-each binds each item to `$row` and renders its children.
    let data = TemplateData::default().with_binding(
        "rows",
        ItemStream::from_items(vec![
            Item::Atomic(AtomValue::String("a".to_owned())),
            Item::Atomic(AtomValue::String("b".to_owned())),
            Item::Atomic(AtomValue::String("c".to_owned())),
        ]),
    );

    let rendered = render_template(
        "{ul | {cem:for-each @select=\"$rows\" @as=\"row\" | {li | {$row}}}}",
        &data,
    );

    assert_eq!(rendered.rendered, "<ul><li>a</li><li>b</li><li>c</li></ul>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_for_each_binds_record_fields_per_item() {
    // Realistic CSS-generator shape: iterate token rows, emit "<token>=<value>" per row.
    let rows = ItemStream::from_items(vec![
        record([
            (
                "token",
                vec![Item::Atomic(AtomValue::String("--cem-gap".to_owned()))],
            ),
            (
                "value",
                vec![Item::Atomic(AtomValue::String("0.5rem".to_owned()))],
            ),
        ]),
        record([
            (
                "token",
                vec![Item::Atomic(AtomValue::String("--cem-inset".to_owned()))],
            ),
            (
                "value",
                vec![Item::Atomic(AtomValue::String("1rem".to_owned()))],
            ),
        ]),
    ]);
    let data = TemplateData::default().with_binding("rows", rows);

    let rendered = render_template(
        "{cem:for-each @select=\"$rows\" @as=\"row\" | {$row.token}={$row.value} }",
        &data,
    );

    assert_eq!(rendered.rendered, "--cem-gap=0.5rem --cem-inset=1rem ");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_for_each_without_select_diagnoses() {
    let rendered = render_template(
        "{cem:for-each @as=\"row\" | {$row}}",
        &TemplateData::default(),
    );

    assert_eq!(rendered.rendered, "");
    assert!(
        rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.for_each_missing_select"),
        "{:?}",
        rendered.diagnostics
    );
}
