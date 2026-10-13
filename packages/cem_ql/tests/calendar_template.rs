use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html, CompileTemplateOptions,
    TemplateData,
};

fn string(value: &str) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::String(value.into()))]
}
fn boolean(value: bool) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::Boolean(value))]
}

#[test]
fn canonical_calendar_retains_named_templates_and_main_body() {
    let declaration =
        include_str!("../../cem-components/src/components/cem-datepicker/cem-datepicker.xhtml");
    let source = declaration
        .split("<template id=\"cem-datepicker\" type=\"text/cem-ml\">")
        .nth(1)
        .unwrap()
        .split("</template>")
        .next()
        .unwrap();
    let compiled = compile_template(source, &CompileTemplateOptions::default());
    assert!(
        compiled.diagnostics.is_empty(),
        "{:?}",
        compiled.diagnostics
    );
    for value in ["0001-01-01", "9999-12-31"] {
        let days = vec![
            Item::Record(
                [
                    ("value".into(), string(value)),
                    ("id".into(), string("selected-day")),
                    ("number".into(), string("1")),
                    ("fullLabel".into(), string("Selected date")),
                    ("selected".into(), boolean(true)),
                    ("current".into(), boolean(true)),
                    ("disabled".into(), boolean(false)),
                    ("active".into(), boolean(true)),
                    ("tabIndex".into(), string("0")),
                ]
                .into(),
            ),
            Item::Record(
                [
                    ("value".into(), string("")),
                    ("id".into(), string("blank-day")),
                ]
                .into(),
            ),
        ];
        let weeks = Item::Array(vec![Item::Record(
            [("days".into(), vec![Item::Array(days)])].into(),
        )]);
        let slices = Item::Record(
            [
                ("weeks".into(), vec![weeks.clone()]),
                ("weekdays".into(), vec![Item::Array(vec![])]),
                ("heading".into(), string("Calendar month")),
                ("headingId".into(), string("month-heading")),
                ("dialogId".into(), string("date-dialog")),
                ("mode".into(), string("valid")),
                ("applyDisabled".into(), boolean(false)),
                ("previousDisabled".into(), boolean(true)),
                ("nextDisabled".into(), boolean(false)),
            ]
            .into(),
        );
        let data = TemplateData::default()
            .with_binding("weeks", ItemStream::once(weeks))
            .with_binding("weekdays", ItemStream::once(Item::Array(vec![])))
            .with_binding(
                "datadom",
                ItemStream::once(Item::Record([("slices".into(), vec![slices])].into())),
            );
        let rendered = render_compiled_template(&compiled, &data);
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        let html = render_plan_to_html(&rendered);
        assert_eq!(html.matches("<dialog ").count(), 1, "{html}");
        assert!(html.contains("Calendar month"), "{html}");
        assert!(html.contains(&format!("data-date=\"{value}\"")), "{html}");
        assert!(html.contains("aria-current=\"date\""), "{html}");
        assert!(html.contains("aria-selected=\"true\""), "{html}");
        let blank = html.find("id=\"blank-day\"").unwrap();
        assert!(
            html[..blank]
                .rsplit('<')
                .next()
                .unwrap()
                .starts_with("span "),
            "{html}"
        );
        assert_eq!(html.matches("role=\"gridcell\"").count(), 2, "{html}");
        assert_eq!(html.matches(" disabled=\"true\"").count(), 1, "{html}");
    }
}
