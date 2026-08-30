use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, CompileTemplateOptions, HostAttributeUpdate,
    RenderPlanNode, TemplateData,
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

    let rendered = render_template("{span | {$ label}}", &data);

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
        r#"{button @class="action {tone}" @disabled="{disabled}" | Save}"#,
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
fn render_template_interpolates_event_coordinates_into_css_lengths() {
    let event_payload = record([
        ("offsetX", vec![Item::Atomic(AtomValue::Integer(157))]),
        ("offsetY", vec![Item::Atomic(AtomValue::Integer(120))]),
    ]);
    let datadom = record([("eventPayloads", vec![record([("s", vec![event_payload])])])]);
    let rendered = render_template(
        r#"{textarea @style="width:16rem;height:16rem;box-shadow:inset
            {datadom.eventPayloads.s.offsetX ?? 0
            }px {datadom.eventPayloads.s.offsetY ?? 0}px gold;"}"#,
        &TemplateData::default().with_binding("datadom", ItemStream::once(datadom)),
    );

    assert_eq!(
        rendered.rendered,
        "<textarea style=\"width:16rem;height:16rem;box-shadow:inset\n            157px 120px gold;\"></textarea>"
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_preserves_explicit_empty_and_boolean_attributes() {
    let rendered = render_template(
        r#"{button @type=button @alt="" @required @data-state="{state}" | Save}"#,
        &TemplateData::default().with_binding("state", string_value("")),
    );

    assert_eq!(
        rendered.rendered,
        r#"<button type="button" alt required>Save</button>"#
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

    let rendered = render_template(r#"{span @title="{label}" | {$ label}}"#, &data);

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
    let rendered = render_template("{span | {$ missing}}", &TemplateData::default());

    assert_eq!(rendered.rendered, "<span></span>");
    assert!(rendered
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));
}

#[test]
fn compiled_template_renders_multiple_snapshots_without_recompile() {
    let artifact = compile_template(
        "{span | {$ label}}",
        &CompileTemplateOptions {
            host_bindings: vec!["label".to_owned()],
            ..CompileTemplateOptions::default()
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
fn compile_template_can_skip_cemt_function_body_expressions() {
    let source = r#"{module |
        {function @name="acme.normalize-callout" @returns="object" |
            {param @name="marker" @type="string"}
            {body |
                {$ missing_binding }
            }
        }
        {template @name="main" |
            {body | {$ marker} }
        }
    }"#;

    let unchecked = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["marker".to_owned()],
            ..CompileTemplateOptions::default()
        },
    );
    assert!(unchecked
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));

    let checked = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["marker".to_owned()],
            skip_cemt_function_bodies: true,
        },
    );
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn render_plan_preserves_structured_nodes_and_source_maps() {
    let artifact = compile_template(
        r#"{button @class="action {tone}" | {$ label}}"#,
        &CompileTemplateOptions {
            host_bindings: vec!["tone".to_owned(), "label".to_owned()],
            ..CompileTemplateOptions::default()
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
        namespace,
        attributes,
        children,
        source_map,
    }] = plan.nodes.as_slice()
    else {
        panic!("expected one rendered element");
    };

    assert_eq!(tag, "button");
    assert_eq!(*namespace, None);
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
    let rendered = render_template("{span | {$ datadom.attributes.label}}", &data);

    assert_eq!(rendered.rendered, "<span>Email</span>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_selects_top_level_host_binding_from_data_document() {
    let input = record([(
        "attributes",
        vec![record([(
            "kind",
            vec![Item::Atomic(AtomValue::String("document".to_owned()))],
        )])],
    )]);
    let data = TemplateData::default().with_binding("input", ItemStream::once(input));

    let rendered = render_template("{span | {$ input.attributes.kind}}", &data);

    assert_eq!(rendered.rendered, "<span>document</span>");
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
        "{span | {$ datadom.attributes.label}-{$ datadom.dataset.variant}-{$ datadom.slices.open}-{$ datadom.payload.text}-{$ datadom.slots.leading}}",
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
        r#"{span | {$ datadom.attributes.missing ?? "fallback"}}"#,
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
fn render_template_coalesces_slot_text_inside_attribute_value_template() {
    let text = record([
        (
            "kind",
            vec![Item::Atomic(AtomValue::String("text".to_owned()))],
        ),
        (
            "text",
            vec![Item::Atomic(AtomValue::String("🥕".to_owned()))],
        ),
    ]);
    let slotted = record([("children", vec![Item::Array(vec![text])])]);
    let supplied_datadom = record([(
        "slots",
        vec![record([("slot2", vec![Item::Array(vec![slotted])])])],
    )]);
    let missing_datadom = record([("slots", vec![record([])])]);
    let source = r#"{input @placeholder='🐇❤️{datadom.slots.slot2.children.text ?? "🐇"}'}"#;

    let supplied = render_template(
        source,
        &TemplateData::default().with_binding("datadom", ItemStream::once(supplied_datadom)),
    );
    assert!(
        supplied.diagnostics.is_empty(),
        "{:?}",
        supplied.diagnostics
    );
    assert_eq!(supplied.rendered, r#"<input placeholder="🐇❤️🥕">"#);

    let missing = render_template(
        source,
        &TemplateData::default().with_binding("datadom", ItemStream::once(missing_datadom)),
    );
    assert!(missing.diagnostics.is_empty(), "{:?}", missing.diagnostics);
    assert_eq!(missing.rendered, r#"<input placeholder="🐇❤️🐇">"#);
}

#[test]
fn render_template_variable_binds_reusable_expression_for_following_content() {
    let row = record([("id", vec![Item::Atomic(AtomValue::String("2".to_owned()))])]);
    let datadom = record([
        (
            "attributes",
            vec![record([(
                "id",
                vec![Item::Atomic(AtomValue::String("1".to_owned()))],
            )])],
        ),
        ("rows", vec![Item::Array(vec![row])]),
    ]);
    let source = r#"
        {cem:variable @name=sprite-base @select='"https://sprites.example.test"'}
        {img @src="{$sprite-base}/{$datadom.attributes.id}.svg"}
        {cem:for-each @select="datadom.rows" @as=item |
            {img @src="{$sprite-base}/{$item.id}.svg"}
        }
    "#;

    let rendered = render_template(
        source,
        &TemplateData::default().with_binding("datadom", ItemStream::once(datadom)),
    );

    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(
        rendered.rendered.split_whitespace().collect::<String>(),
        r#"<imgsrc="https://sprites.example.test/1.svg"><imgsrc="https://sprites.example.test/2.svg">"#,
    );
}

#[test]
fn render_template_variable_shadows_only_inside_its_surrounding_scope() {
    let rendered = render_template(
        r#"{cem:variable @name=value @select='"outer"'}{div |{cem:variable @name=value @select='"inner"'}{span |{$value}}}{span |{$value}}"#,
        &TemplateData::default(),
    );

    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(
        rendered.rendered,
        "<div><span>inner</span></div><span>outer</span>"
    );
}

#[test]
fn render_template_coalesce_prefers_present_selection() {
    let data = TemplateData::default().with_binding("label", string_value("Email"));

    let rendered = render_template(
        r#"{span | {$ datadom.attributes.label ?? "fallback"}}"#,
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
        r#"{span | {$ datadom.attributes.missing ?? datadom.attributes.alt ?? "fallback"}}"#,
        &data,
    );

    assert_eq!(rendered.rendered, "<span>Alt</span>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn attribute_declarations_return_default_and_selected_host_updates() {
    let template = concat!(
        r#"{attribute @name=p1 | default_P1}"#,
        r#"{attribute @name=p2 @select='"always_p2"'}"#,
        r#"{attribute @name=p3 @select='datadom.attributes.p3 ?? "def_P3"'}"#,
        r#"{p | {$p1}|{$p2}|{$p3}|{$datadom.attributes.p1}|{$datadom.attributes.p2}|{$datadom.attributes.p3}}"#,
    );

    let defaults = render_template(template, &TemplateData::default());
    assert_eq!(
        defaults.rendered,
        "<p>default_P1|always_p2|def_P3|default_P1|always_p2|def_P3</p>",
        "{:?}",
        defaults.diagnostics
    );
    assert_eq!(
        defaults.host_attribute_updates,
        vec![
            HostAttributeUpdate::new("p1", "default_P1"),
            HostAttributeUpdate::new("p2", "always_p2"),
            HostAttributeUpdate::new("p3", "def_P3"),
        ]
    );
    assert!(
        defaults.diagnostics.is_empty(),
        "{:?}",
        defaults.diagnostics
    );

    let overridden = render_template(
        template,
        &TemplateData::default()
            .with_binding("p1", string_value("123"))
            .with_binding("p2", string_value("ignored"))
            .with_binding("p3", string_value("qwe")),
    );
    assert_eq!(
        overridden.rendered,
        "<p>123|always_p2|qwe|123|always_p2|qwe</p>"
    );
    assert_eq!(
        overridden.host_attribute_updates,
        vec![
            HostAttributeUpdate::new("p2", "always_p2"),
            HostAttributeUpdate::new("p3", "qwe"),
        ]
    );
    assert!(
        overridden.diagnostics.is_empty(),
        "{:?}",
        overridden.diagnostics
    );
}

#[test]
fn selected_attributes_derive_boolean_and_slice_event_values() {
    let template = concat!(
        r#"{attribute @name=is-changed @select='if datadom.eventPayloads.s { true } else { false }'}"#,
        r#"{attribute @name=v @select='if datadom.eventPayloads.s { datadom.slices.s } else { datadom.attributes.v ?? "def" }'}"#,
        r#"{p | {$datadom.attributes.v}|{$v}|{$datadom.attributes.is-changed}|{$is-changed}}"#,
    );
    let datadom = record([
        (
            "attributes",
            vec![record([(
                "v",
                vec![Item::Atomic(AtomValue::String("From Container".to_owned()))],
            )])],
        ),
        (
            "slices",
            vec![record([(
                "s",
                vec![Item::Atomic(AtomValue::String("From Slice".to_owned()))],
            )])],
        ),
        (
            "eventPayloads",
            vec![record([(
                "s",
                vec![record([(
                    "type",
                    vec![Item::Atomic(AtomValue::String("input".to_owned()))],
                )])],
            )])],
        ),
    ]);

    let rendered = render_template(
        template,
        &TemplateData::default()
            .with_binding("v", string_value("From Container"))
            .with_binding("datadom", ItemStream::once(datadom)),
    );

    assert_eq!(
        rendered.rendered,
        "<p>From Slice|From Slice|true|true</p>"
    );
    assert_eq!(
        rendered.host_attribute_updates,
        vec![
            HostAttributeUpdate::new("is-changed", "true"),
            HostAttributeUpdate::new("v", "From Slice"),
        ]
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn invalid_attribute_select_preserves_host_binding_without_a_host_update() {
    let rendered = render_template(
        r#"{attribute @name=label @select="missing +"}{p | {$label}|{$datadom.attributes.label}}"#,
        &TemplateData::default().with_binding("label", string_value("Authored")),
    );

    assert_eq!(rendered.rendered, "<p>Authored|Authored</p>");
    assert!(rendered.host_attribute_updates.is_empty());
    assert!(rendered
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));
}

#[test]
fn ordinary_select_attributes_remain_literal_attribute_values() {
    let rendered = render_template(
        r#"{button @select="raw-value" | Choose}"#,
        &TemplateData::default(),
    );

    assert_eq!(
        rendered.rendered,
        r#"<button select="raw-value">Choose</button>"#
    );
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
        r#"{cem:if @test="datadom.attributes.label" | {span | {$ datadom.attributes.label}}}"#,
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
    // appear in render output; only the `<button>` (with its resolved `label`) renders.
    let data = TemplateData::default().with_binding("label", string_value("Save"));
    let rendered = render_template(
        r#"{attribute @name="label" | Save}{slice @name="open"}{button @type=button | {$ label}}"#,
        &data,
    );

    assert_eq!(rendered.rendered, r#"<button type="button">Save</button>"#);
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_applies_declaration_defaults() {
    let template = r#"{attribute @name="label" | Save}{button @type=button | {$ label}}"#;

    // No host data -> the declared default seeds `label` (the render engine owns defaults).
    let default_render = render_template(template, &TemplateData::default());
    assert_eq!(
        default_render.rendered,
        r#"<button type="button">Save</button>"#
    );
    assert!(
        default_render.diagnostics.is_empty(),
        "{:?}",
        default_render.diagnostics
    );

    // A host-provided value overrides the declared default.
    let override_render = render_template(
        template,
        &TemplateData::default().with_binding("label", string_value("Submit")),
    );
    assert_eq!(
        override_render.rendered,
        r#"<button type="button">Submit</button>"#
    );
    assert!(
        override_render.diagnostics.is_empty(),
        "{:?}",
        override_render.diagnostics
    );
}

#[test]
fn declared_slice_defaults_update_the_current_data_document() {
    let rendered = render_template(
        r#"{slice @name="s" | xB}{input @value="{str:substring(datadom.slices.s, 2)}"}{output | {$s}|{$datadom.slices.s}}"#,
        &TemplateData::default(),
    );

    assert_eq!(
        rendered.rendered,
        r#"<input value="B"><output>xB|xB</output>"#
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn declared_boolean_slice_defaults_keep_their_boolean_type() {
    let rendered = render_template(
        r#"{slice @name="open" | false}{output | {$open}|{$datadom.slices.open}}{cem:if @test="datadom.slices.open" | {p | must stay hidden}}"#,
        &TemplateData::default(),
    );

    assert_eq!(rendered.rendered, "<output>false|false</output>");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_constructs_dynamic_elements_and_attributes() {
    let rendered = render_template(
        r#"{element @name="{tag}" |{attribute @name="data-id" @value="{id}"}{attribute @name="{dynamicName}" | {$ dynamicValue}}{$ label}}"#,
        &TemplateData::default()
            .with_binding("tag", string_value("article"))
            .with_binding("id", string_value("42"))
            .with_binding("dynamicName", string_value("aria-label"))
            .with_binding("dynamicValue", string_value("Read"))
            .with_binding("label", string_value("Title")),
    );

    assert_eq!(
        rendered.rendered,
        r#"<article aria-label="Read" data-id="42">Title</article>"#
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_attaches_conditional_attributes_to_parent() {
    let rendered = render_template(
        r#"{input @type="email" |{cem:if @test="required" |{attribute @name="required"}}}"#,
        &TemplateData::default().with_binding("required", bool_value(true)),
    );

    assert_eq!(rendered.rendered, r#"<input type="email" required>"#);
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
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
    assert!(
        nested_b.diagnostics.is_empty(),
        "{:?}",
        nested_b.diagnostics
    );

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
    // A multi-item host binding; for-each binds each item to `row` and renders its children.
    let data = TemplateData::default().with_binding(
        "rows",
        ItemStream::from_items(vec![
            Item::Atomic(AtomValue::String("a".to_owned())),
            Item::Atomic(AtomValue::String("b".to_owned())),
            Item::Atomic(AtomValue::String("c".to_owned())),
        ]),
    );

    let rendered = render_template(
        "{ul | {cem:for-each @select=\"rows\" @as=\"row\" | {li | {$ row}}}}",
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
fn render_template_for_each_binds_position() {
    // XSLT `position()` parity: the legacy bridge rewrites `position()` to `position`, which
    // cem:for-each binds to the 1-based iteration index.
    let data = TemplateData::default().with_binding(
        "rows",
        ItemStream::from_items(vec![
            Item::Atomic(AtomValue::String("a".to_owned())),
            Item::Atomic(AtomValue::String("b".to_owned())),
            Item::Atomic(AtomValue::String("c".to_owned())),
        ]),
    );

    let rendered = render_template(
        "{cem:for-each @select=\"rows\" @as=\"row\" | {$ position}:{$ row};}",
        &data,
    );

    assert_eq!(rendered.rendered, "1:a;2:b;3:c;");
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
        "{cem:for-each @select=\"rows\" @as=\"row\" | {$ row.token}={$ row.value} }",
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
fn render_template_for_each_iterates_array_item_members() {
    // The host data-document delivers a slice (token rows shaped from a `<table>`) as a single
    // `Item::Array` — exactly what the WASM JSON boundary builds from a JSON array of objects
    // under `datadom.slices.<name>`. for-each must iterate the array members, not the array.
    let rows = Item::Array(vec![
        record([
            (
                "td1",
                vec![Item::Atomic(AtomValue::String("--cem-gap".to_owned()))],
            ),
            (
                "td2",
                vec![Item::Atomic(AtomValue::String("0.5rem".to_owned()))],
            ),
        ]),
        record([
            (
                "td1",
                vec![Item::Atomic(AtomValue::String("--cem-inset".to_owned()))],
            ),
            (
                "td2",
                vec![Item::Atomic(AtomValue::String("1rem".to_owned()))],
            ),
        ]),
    ]);
    let datadom = record([("slices", vec![record([("geometry", vec![rows])])])]);
    let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

    let rendered = render_template(
        "{cem:for-each @select=\"datadom.slices.geometry\" @as=\"row\" | {$ row.td1}: {$ row.td2};}",
        &data,
    );

    assert_eq!(rendered.rendered, "--cem-gap: 0.5rem;--cem-inset: 1rem;");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_recursively_exposes_every_payload_attribute_without_a_whitelist() {
    let text = record([
        (
            "kind",
            vec![Item::Atomic(AtomValue::String("text".to_owned()))],
        ),
        (
            "text",
            vec![Item::Atomic(AtomValue::String("Hello".to_owned()))],
        ),
    ]);
    let node = record([
        (
            "kind",
            vec![Item::Atomic(AtomValue::String("element".to_owned()))],
        ),
        (
            "tag",
            vec![Item::Atomic(AtomValue::String("strong".to_owned()))],
        ),
        (
            "attributes",
            vec![record([
                (
                    "data-flavor",
                    vec![Item::Atomic(AtomValue::String("pear".to_owned()))],
                ),
                (
                    "slot",
                    vec![Item::Atomic(AtomValue::String("heading".to_owned()))],
                ),
                (
                    "title",
                    vec![Item::Atomic(AtomValue::String("Complete".to_owned()))],
                ),
            ])],
        ),
        ("children", vec![Item::Array(vec![text])]),
    ]);
    let payload = record([("nodes", vec![Item::Array(vec![node])])]);
    let datadom = record([("payload", vec![payload])]);
    let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));
    let template = r#"
        {module |
            {template @name=node |
                {param @name=node}
                {body |
                    {cem:choose |
                        {cem:when @test='node.kind == "element"' |
                            {details @open=open |
                                {summary |
                                    {b | {$node.tag}}
                                    {cem:for-each @select="record:entries(node.attributes)" @as=attribute |
                                        {code | {$ attribute.key + "=\"" + attribute.value + "\""}}
                                    }
                                }
                                {cem:for-each @select="$node.children" @as=child |
                                    {call @template=node @with:node="{$child}"}
                                }
                            }
                        }
                        {cem:when @test='node.kind == "text"' | {p | {$node.text}}}
                    }
                }
            }
            {body |
                {details @open=open |
                    {summary | {b | template}}
                    {cem:for-each @select="$datadom.payload.nodes" @as=node |
                        {call @template=node @with:node="{$node}"}
                    }
                }
            }
        }
    "#;

    let rendered = render_template(template, &data);

    for expected in [
        "<b>template</b>",
        "<b>strong</b>",
        "<code>data-flavor=\"pear\"</code>",
        "<code>slot=\"heading\"</code>",
        "<code>title=\"Complete\"</code>",
        "<p>Hello</p>",
    ] {
        assert!(
            rendered.rendered.contains(expected),
            "missing {expected:?} in {}; diagnostics: {:?}",
            rendered.rendered,
            rendered.diagnostics
        );
    }
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_http_resource_envelope_drives_for_each() {
    // Phase 1 resource hosts retain transport/lifecycle ownership and expose the
    // completed stream-derived projection under the resource envelope's `data`
    // field. CEM-QL owns interpolation of the control declaration and iteration
    // over that projection; the browser adapter must not reimplement either.
    let results = Item::Array(vec![
        record([
            (
                "name",
                vec![Item::Atomic(AtomValue::String("Bulbasaur".to_owned()))],
            ),
            (
                "status",
                vec![Item::Atomic(AtomValue::String("ready".to_owned()))],
            ),
        ]),
        record([
            (
                "name",
                vec![Item::Atomic(AtomValue::String("Ivysaur".to_owned()))],
            ),
            (
                "status",
                vec![Item::Atomic(AtomValue::String("waiting".to_owned()))],
            ),
        ]),
    ]);
    let page = record([
        (
            "state",
            vec![Item::Atomic(AtomValue::String("loaded".to_owned()))],
        ),
        ("data", vec![record([("results", vec![results])])]),
    ]);
    let datadom = record([("slices", vec![record([("page", vec![page])])])]);
    let data = TemplateData::default()
        .with_binding("datadom", ItemStream::once(datadom))
        .with_binding("resource_url", string_value("./pokemon.json"));

    let rendered = render_template(
        concat!(
            "{http-request @slice=page @url=\"{$resource_url}\" @content-type=\"application/json\"}",
            "{ul |{cem:for-each @select=\"datadom.slices.page.data.results\" @as=record |",
            "{li @data-status=\"{$record.status}\" | {$ record.name}}}}"
        ),
        &data,
    );

    assert!(
        rendered.rendered.starts_with(
            "<http-request slice=\"page\" url=\"./pokemon.json\" content-type=\"application/json\"></http-request>"
        ),
        "{}",
        rendered.rendered
    );
    assert!(
        rendered
            .rendered
            .ends_with("<ul><li data-status=\"ready\">Bulbasaur</li><li data-status=\"waiting\">Ivysaur</li></ul>"),
        "{}",
        rendered.rendered
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_rich_content_emits_literal_braces_around_for_each() {
    // The CSS-generator shape: rich-content (triple backtick) supplies the literal `:root { … }`
    // braces that would otherwise collide with cem-ml structure, while a sibling for-each emits the
    // dynamic declaration lines from an array slice.
    let rows = Item::Array(vec![
        record([
            (
                "td1",
                vec![Item::Atomic(AtomValue::String(
                    "--cem-control-height".to_owned(),
                ))],
            ),
            (
                "td2",
                vec![Item::Atomic(AtomValue::String("2.5rem".to_owned()))],
            ),
        ]),
        record([
            (
                "td1",
                vec![Item::Atomic(AtomValue::String(
                    "--cem-list-row-height".to_owned(),
                ))],
            ),
            (
                "td2",
                vec![Item::Atomic(AtomValue::String("3rem".to_owned()))],
            ),
        ]),
    ]);
    let datadom = record([("slices", vec![record([("geometry", vec![rows])])])]);
    let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

    // Leading whitespace after `|` is trimmed (relaxed content boundary), so per-row newlines come
    // from a rich-content fence at the head of the for-each body, not bare indentation.
    let rendered = render_template(
        "{code |```:root {```{cem:for-each @select=\"datadom.slices.geometry\" @as=\"row\" |```\n  ```{$ row.td1}: {$ row.td2};}```\n}```}",
        &data,
    );

    assert_eq!(
        rendered.rendered,
        "<code>:root {\n  --cem-control-height: 2.5rem;\n  --cem-list-row-height: 3rem;\n}</code>"
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_for_each_with_cem_if_tier_filter() {
    // CSS-generator tier filtering: iterate token rows but emit only the non-deprecated ones —
    // the cem-ml equivalent of the legacy XPath predicate `tr[normalize-space(td[4]) != 'deprecated']`.
    let rows = Item::Array(vec![
        record([
            (
                "td1",
                vec![Item::Atomic(AtomValue::String(
                    "--cem-layout-keep".to_owned(),
                ))],
            ),
            (
                "td2",
                vec![Item::Atomic(AtomValue::String("1rem".to_owned()))],
            ),
            (
                "td4",
                vec![Item::Atomic(AtomValue::String("recommended".to_owned()))],
            ),
        ]),
        record([
            (
                "td1",
                vec![Item::Atomic(AtomValue::String(
                    "--cem-layout-drop".to_owned(),
                ))],
            ),
            (
                "td2",
                vec![Item::Atomic(AtomValue::String("2rem".to_owned()))],
            ),
            (
                "td4",
                vec![Item::Atomic(AtomValue::String("deprecated".to_owned()))],
            ),
        ]),
    ]);
    let datadom = record([("slices", vec![record([("layout", vec![rows])])])]);
    let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

    // `@test` is whole-expression cem-ql: use bare binding names and a double-quoted
    // cem-ql string literal, so the `@test` attribute is single-quoted to avoid a quote clash.
    let rendered = render_template(
        "{cem:for-each @select=\"datadom.slices.layout\" @as=\"row\" |{cem:if @test='row.td4 != \"deprecated\"' |```\n  ```{$ row.td1}: {$ row.td2};}}",
        &data,
    );

    assert_eq!(rendered.rendered, "\n  --cem-layout-keep: 1rem;");
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_action_cross_product_with_emotion_substitution() {
    // cem-colors shape: intent×state nested for-each emitting cross-product token names, with the
    // `[emotion]` placeholder in the state formula replaced by the intent's emotion via str:replace.
    let intents = Item::Array(vec![record([
        (
            "td1",
            vec![Item::Atomic(AtomValue::String("explicit".to_owned()))],
        ),
        (
            "td2",
            vec![Item::Atomic(AtomValue::String("trust".to_owned()))],
        ),
    ])]);
    let states = Item::Array(vec![record([
        (
            "td1",
            vec![Item::Atomic(AtomValue::String("disabled".to_owned()))],
        ),
        (
            "td2",
            vec![Item::Atomic(AtomValue::String(
                "mix(var(--cem-palette-[emotion]), var(--cem-palette-[emotion]-x))".to_owned(),
            ))],
        ),
    ])]);
    let datadom = record([(
        "slices",
        vec![record([
            ("intents", vec![intents]),
            ("states", vec![states]),
        ])],
    )]);
    let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

    let rendered = render_template(
        "{cem:for-each @select=\"datadom.slices.intents\" @as=\"intent\" |{cem:for-each @select=\"datadom.slices.states\" @as=\"state\" |--cem-action-{$ intent.td1}-{$ state.td1}-background: {$ str:replace(state.td2, \"[emotion]\", intent.td2)};}}",
        &data,
    );

    assert_eq!(
        rendered.rendered,
        "--cem-action-explicit-disabled-background: mix(var(--cem-palette-trust), var(--cem-palette-trust-x));"
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_cross_table_join_resolves_palette_reference() {
    // cem-colors emotion-shift: choose `--cem-color-<name>` when that token exists in the
    // hue-variant table, else fall back to `--cem-palette-<name>`. The existence check projects a
    // field across an array slice (`datadom.slices.hue.td1`, flattened) and tests existential `==`
    // against a string-`+`-built target.
    let hue = Item::Array(vec![
        record([(
            "td1",
            vec![Item::Atomic(AtomValue::String(
                "--cem-color-cyan-xl".to_owned(),
            ))],
        )]),
        record([(
            "td1",
            vec![Item::Atomic(AtomValue::String(
                "--cem-color-blue-xl".to_owned(),
            ))],
        )]),
    ]);
    let shift = Item::Array(vec![record([
        (
            "td1",
            vec![Item::Atomic(AtomValue::String(
                "--cem-palette-comfort".to_owned(),
            ))],
        ),
        (
            "td3",
            vec![Item::Atomic(AtomValue::String("cyan-xl".to_owned()))],
        ),
        (
            "td4",
            vec![Item::Atomic(AtomValue::String("warm".to_owned()))],
        ),
    ])]);
    let datadom = record([(
        "slices",
        vec![record([("hue", vec![hue]), ("shift", vec![shift])])],
    )]);
    let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

    let rendered = render_template(
        "{cem:for-each @select=\"datadom.slices.shift\" @as=\"emo\" |{$ emo.td1}: light-dark({cem:choose |{cem:when @test='datadom.slices.hue.td1 == \"--cem-color-\" + emo.td3' |var(--cem-color-{$ emo.td3})}{cem:otherwise |var(--cem-palette-{$ emo.td3})}}, {cem:choose |{cem:when @test='datadom.slices.hue.td1 == \"--cem-color-\" + emo.td4' |var(--cem-color-{$ emo.td4})}{cem:otherwise |var(--cem-palette-{$ emo.td4})}});}",
        &data,
    );

    assert_eq!(
        rendered.rendered,
        "--cem-palette-comfort: light-dark(var(--cem-color-cyan-xl), var(--cem-palette-warm));"
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
}

#[test]
fn render_template_for_each_without_select_diagnoses() {
    let rendered = render_template(
        "{cem:for-each @as=\"row\" | {$ row}}",
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
