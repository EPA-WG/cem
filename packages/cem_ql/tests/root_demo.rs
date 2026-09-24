//! The authored gallery selects payload entries without changing the broad host lookup.
use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};

fn record(fields: Vec<(&str, Vec<Item>)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(key, values)| (key.into(), values))
            .collect(),
    )
}

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn pokemon(id: &str, name: &str) -> Item {
    record(vec![
        ("text", vec![string(name)]),
        (
            "attributes",
            vec![record(vec![("pokemon-id", vec![string(id)])])],
        ),
    ])
}

#[test]
fn root_gallery_pokemon_loop_selects_only_payload_entries() {
    let page = include_str!("../../cem-elements/index.html");
    let template = page
        .split("<cem-element tag=\"pokemon-tile\"")
        .nth(1)
        .unwrap()
        .split("<template type=\"text/cem-ml\">")
        .nth(1)
        .unwrap()
        .split("</template>")
        .next()
        .unwrap();
    for entries in [
        vec![("2", "ivysaur"), ("3", "venusaur")],
        vec![("37", "vulpix")],
        vec![],
    ] {
        let payload: Vec<Item> = entries.iter().map(|(id, name)| pokemon(id, name)).collect();
        let mut broad = payload.clone();
        broad.push(pokemon("1", ""));
        let data = TemplateData::default().with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "attributes",
                    vec![record(vec![
                        ("title", vec![string("bulbasaur")]),
                        ("pokemon-id", vec![string("1")]),
                    ])],
                ),
                ("slices", vec![record(vec![])]),
                ("dataset", vec![record(vec![])]),
                (
                    "payload",
                    vec![record(vec![(
                        "elementsByAttribute",
                        vec![record(vec![("pokemon-id", payload)])],
                    )])],
                ),
                (
                    "elementsByAttribute",
                    vec![record(vec![("pokemon-id", broad)])],
                ),
            ])),
        );
        let rendered = render_template(template, &data);
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        assert_eq!(
            rendered.rendered.matches("<button").count(),
            entries.len(),
            "{}",
            rendered.rendered
        );
        for (id, name) in &entries {
            assert!(
                rendered
                    .rendered
                    .contains(&format!("/{id}.svg\" alt=\"{name}\"")),
                "{}",
                rendered.rendered
            );
        }
        let control = render_template(
            "{cem:for-each @select=datadom.elementsByAttribute.pokemon-id @as=item | {button | {$item.text}}}",
            &data,
        );
        assert!(control.diagnostics.is_empty(), "{:?}", control.diagnostics);
        assert_eq!(
            control.rendered.matches("<button").count(),
            entries.len() + 1
        );
    }
}
