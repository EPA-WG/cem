//! Imported presentation rules change cells, not the source tree or table renderer.
use cem_ml::{
    import::import_data,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    value::artifact::CemValueArtifactLimits,
};
use cem_ql::{
    eval::{imported_cem_tree, AtomValue, Item, ItemStream},
    render::{
        compile_template_module_closure, project_render_plan_with_control,
        render_compiled_template, CompileTemplateOptions, RenderPlan, RenderPlanNode, TemplateData,
        TemplateModuleClosure, TemplateModuleSource,
    },
};

const BASE: &str = include_str!("../../cem-elements/demo/data-table-view.cemt");
fn pokemon_template() -> &'static str {
    include_str!("../../cem-elements/demo/cell-overrides.html")
        .split_once("<template type=\"text/cem-ml\">")
        .unwrap()
        .1
        .split_once("</template>")
        .unwrap()
        .0
}
const STOCK: &str = include_str!("../../cem-elements/demo/stock-cell.cemt");
const LIST: &str = include_str!("../../cem-elements/demo/pokemon-cells.json");
const PRODUCTS: &str = include_str!("../../cem-elements/demo/stock-cells.xml");

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}
fn record(fields: Vec<(&str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), vec![value]))
            .collect(),
    )
}
fn render_native(template: &str, source: &str, mut slices: Vec<(&str, Item)>) -> RenderPlan {
    let hash = |text: &str| {
        cem_ml::content_cache::ContentHash::from_blake3(text.as_bytes()).header_value()
    };
    // HTTP data enters through the shared import boundary, just as in the browser.
    let format = if template == pokemon_template() {
        "json"
    } else {
        "xml"
    };
    let payload = if template == pokemon_template() || template == STOCK {
        if !slices.iter().any(|(name, _)| *name == "catalog") {
            let catalog =
                match import_data(source, format, "cem", "https://example.test/demo/catalog") {
                    Ok(owner) => record(vec![
                        ("state", string("loaded")),
                        ("data", imported_cem_tree(owner)),
                    ]),
                    Err(_) => record(vec![("state", string("failed"))]),
                };
            slices.push(("catalog", catalog));
        }
        ""
    } else {
        source
    };
    let data = TemplateData::default()
        .with_binding("format", ItemStream::once(string(format)))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    record(vec![("nodes", record(vec![("text", string(payload))]))]),
                ),
                ("slices", record(slices)),
            ])),
        );
    // Browser import discovery runs this shared preflight before compiling the closure.
    let preflight = cem_ml::transform_template::parse_cem_native_template_module_options(
        cem_ml::transform_template::TransformTemplateModuleParseRequest {
            template: cem_ml::engine::TemplateInput {
                uri: "https://example.test/demo/cell.cemt".into(),
                bytes: template.as_bytes().to_vec(),
                identity: None,
                root_scope: Default::default(),
            },
        },
    );
    assert!(
        preflight.diagnostics.is_empty(),
        "{:?}",
        preflight.diagnostics
    );
    let artifact = compile_template_module_closure(
        template,
        &TemplateModuleClosure {
            root_uri: "https://example.test/demo/cell.cemt".into(),
            root_content_hash: hash(template),
            modules: vec![TemplateModuleSource {
                alias: "base".into(),
                parent_uri: None,
                uri: "https://example.test/demo/data-table-view.cemt".into(),
                content_hash: hash(BASE),
                source: BASE.into(),
            }],
            ..Default::default()
        },
        &CompileTemplateOptions::default(),
    );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    plan
}
fn render(template: &str, source: &str, slices: Vec<(&str, Item)>) -> RenderPlan {
    // Presentation assertions inspect the final projection. The native plan keeps
    // references and attribute value streams until this explicit boundary.
    project_render_plan_with_control(
        &render_native(template, source, slices),
        cem_ql::eval::QueryContextScope(0),
        &CemValueArtifactLimits::default(),
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    )
    .unwrap()
}
fn children(node: &RenderPlanNode) -> &[RenderPlanNode] {
    match node {
        RenderPlanNode::Element { children, .. } => children,
        _ => &[],
    }
}
fn tag(node: &RenderPlanNode) -> &str {
    match node {
        RenderPlanNode::Element { tag, .. } => tag,
        _ => "",
    }
}
fn attr<'a>(node: &'a RenderPlanNode, name: &str) -> Option<&'a str> {
    match node {
        RenderPlanNode::Element { attributes, .. } => attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str()),
        _ => None,
    }
}
fn text(node: &RenderPlanNode) -> String {
    match node {
        RenderPlanNode::Text { text, .. } => text.clone(),
        _ => children(node).iter().map(text).collect(),
    }
}
fn find<'a>(nodes: &'a [RenderPlanNode], name: &str) -> Vec<&'a RenderPlanNode> {
    let mut result = vec![];
    for node in nodes {
        if tag(node) == name {
            result.push(node);
        }
        result.extend(find(children(node), name));
    }
    result
}

#[test]
fn inline_name_rule_renders_json_images_and_base_columns() {
    let plan = render(pokemon_template(), LIST, vec![]);
    let images = find(&plan.nodes, "img");
    assert_eq!(images.len(), 10);
    for (index, image) in images.iter().enumerate() {
        assert_eq!(
            attr(image, "src"),
            Some(format!("https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/{}.svg", index + 1).as_str())
        );
    }
    assert_eq!(attr(images[0], "alt"), Some("bulbasaur"));
    let table = find(&plan.nodes, "table")[0];
    assert_eq!(
        find(children(find(children(table), "thead")[0]), "th")
            .iter()
            .map(|n| text(n).trim().to_owned())
            .collect::<Vec<_>>(),
        ["✓", "name", "url"]
    );
    assert!(text(table).contains("https://pokeapi.co/api/v2/pokemon/1/"));
    assert!(find(&plan.nodes, "textarea").is_empty());
    assert!(!find(&plan.nodes, "button")
        .iter()
        .any(|n| attr(n, "aria-label") == Some("Reset source")));
}

#[test]
fn request_states_do_not_display_stale_tables() {
    for (template, source, format) in [(pokemon_template(), LIST, "json"), (STOCK, PRODUCTS, "xml")]
    {
        for (state, role) in [("", "status"), ("loading", "status"), ("failed", "alert")] {
            let stale =
                imported_cem_tree(import_data(source, format, "cem", "memory:stale").unwrap());
            let plan = render(
                template,
                source,
                vec![(
                    "catalog",
                    if state.is_empty() { record(vec![]) } else { record(vec![("state", string(state)), ("data", stale)]) },
                )],
            );
            assert!(find(&plan.nodes, "table").is_empty());
            assert!(find(&plan.nodes, "img").is_empty());
            assert!(find(&plan.nodes, "p")
                .iter()
                .any(|n| attr(n, "role") == Some(role)));
        }
    }
}

#[test]
fn source_identity_and_name_sorting_survive_cell_overrides() {
    let document = imported_cem_tree(import_data(LIST, "json", "cem", "memory:catalog").unwrap());
    let catalog = record(vec![("state", string("loaded")), ("data", document)]);
    let initial = render(pokemon_template(), LIST, vec![("catalog", catalog.clone())]);
    let table = find(&initial.nodes, "table")[0];
    let button = find(children(find(children(table), "tbody")[0]), "button")[0];
    let selected = attr(button, "value").unwrap();
    assert!(!selected.is_empty());
    let plan = render(
        pokemon_template(),
        LIST,
        vec![
            ("catalog", catalog),
            ("column", string("name")),
            ("mode", string("text")),
            ("selected", string(selected)),
        ],
    );
    let names = find(&plan.nodes, "img")
        .iter()
        .map(|n| attr(n, "alt").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "blastoise",
            "bulbasaur",
            "caterpie",
            "charizard",
            "charmander",
            "charmeleon",
            "ivysaur",
            "squirtle",
            "venusaur",
            "wartortle"
        ]
    );
    let row = find(&plan.nodes, "tr")
        .into_iter()
        .find(|n| attr(n, "aria-selected") == Some("true"))
        .unwrap();
    assert!(text(row).contains("bulbasaur"));
}

#[test]
fn name_match_is_independent_of_path_and_other_fields_fall_back() {
    let source = LIST.replace("results", "other");
    assert_eq!(
        find(&render(pokemon_template(), &source, vec![]).nodes, "img").len(),
        10
    );
    let source = LIST.replace("\"name\"", "\"title\"");
    let plan = render(pokemon_template(), &source, vec![]);
    assert!(find(&plan.nodes, "img").is_empty());
    assert!(text(find(&plan.nodes, "table")[0]).contains("ivysaur"));
}

#[test]
fn zero_stock_rule_is_conditional_and_scoped_to_five_product_rows() {
    let plan = render(STOCK, PRODUCTS, vec![]);
    let warnings = find(&plan.nodes, "strong");
    assert_eq!(warnings.len(), 1);
    assert_eq!(text(warnings[0]).trim(), "Out of stock (0)");
    let table = find(&plan.nodes, "table")[0];
    assert_eq!(
        find(children(find(children(table), "tbody")[0]), "tr").len(),
        5
    );
    assert!(text(table).contains('5'));
    assert!(find(&plan.nodes, "textarea").is_empty());
    for source in [
        PRODUCTS.replace("<stock>0</stock>", "<stock>7</stock>"),
        PRODUCTS.replace(
            "<stock>0</stock>",
            "<p:stock xmlns:p='urn:other'>0</p:stock>",
        ),
        PRODUCTS
            .replace("<product>", "<other>")
            .replace("</product>", "</other>"),
        PRODUCTS
            .replace("<product>", "<p:product xmlns:p='urn:other'>")
            .replace("</product>", "</p:product>"),
        PRODUCTS
            .replace("<catalog>", "<p:catalog xmlns:p='urn:other'>")
            .replace("</catalog>", "</p:catalog>"),
        format!("<wrapper>{PRODUCTS}</wrapper>"),
        PRODUCTS.replace("<stock>0</stock>", "<stock><amount>0</amount></stock>"),
    ] {
        assert!(find(&render(STOCK, &source, vec![]).nodes, "strong").is_empty());
    }
    let padded = PRODUCTS.replace("<stock>0</stock>", "<stock> 0 </stock>");
    assert_eq!(find(&render(STOCK, &padded, vec![]).nodes, "strong").len(), 1);
}

#[test]
fn source_errors_clear_override_output_and_repair_restores_it() {
    for template in [pokemon_template(), STOCK] {
        let plan = render(template, "<broken>", vec![]);
        assert!(find(&plan.nodes, "table").is_empty());
        assert!(find(&plan.nodes, "img").is_empty());
        assert!(find(&plan.nodes, "p")
            .iter()
            .any(|n| attr(n, "role") == Some("alert")));
    }
    assert_eq!(
        find(&render(pokemon_template(), LIST, vec![]).nodes, "img").len(),
        10
    );
}

#[test]
fn nested_collections_keep_base_structure_and_apply_the_selected_rule() {
    let source = r#"[{"name":"ivysaur","url":"https://pokeapi.co/api/v2/pokemon/2/","related":[{"name":"venusaur","url":"https://pokeapi.co/api/v2/pokemon/3/"}]}]"#;
    let plan = render(pokemon_template(), source, vec![]);
    assert_eq!(find(&plan.nodes, "table").len(), 2);
    assert_eq!(find(&plan.nodes, "img").len(), 2);
    let source = PRODUCTS.replace("<stock>5</stock>",
        "<stock>5</stock><suppliers><supplier><stock>0</stock></supplier><supplier><stock>0</stock></supplier></suppliers>");
    let plan = render(STOCK, &source, vec![]);
    assert_eq!(find(&plan.nodes, "table").len(), 2);
    assert_eq!(find(&plan.nodes, "strong").len(), 1);
    assert!(find(children(find(&plan.nodes, "table")[1]), "strong").is_empty());
}

#[test]
fn name_body_reuses_the_same_native_node_as_the_alt_attribute() {
    let native = render_native(pokemon_template(), LIST, vec![]);
    let label = find(&native.nodes, "span")
        .into_iter()
        .find(|n| attr(n, "class") == Some("pokemon-name"))
        .unwrap();
    let target = children(label)
        .iter()
        .find_map(|n| match n {
            RenderPlanNode::Reference { reference, .. } => reference.values().first(),
            _ => None,
        })
        .unwrap();
    let RenderPlanNode::Element { attributes, .. } = find(children(label), "img")[0] else {
        panic!()
    };
    let alt = attributes.iter().find(|a| a.name == "alt").unwrap();
    assert_eq!(alt.value_stream.items.len(), 1);
    assert_eq!(target.identity(), alt.value_stream.items[0].identity());
    assert!(!target.source_map().unwrap().frames.is_empty());
    assert_eq!(
        target.view().unwrap().field("name").unwrap()[0].atom(),
        Some(AtomValue::String("string".into()))
    );
    let parent = target
        .view()
        .unwrap()
        .parent(cem_ql::eval::QueryContextScope(0))
        .unwrap()
        .unwrap();
    assert_eq!(
        parent.view().unwrap().field("name").unwrap()[0].atom(),
        Some(AtomValue::String("property".into()))
    );
    let projected = render(pokemon_template(), LIST, vec![]);
    assert_eq!(
        attr(find(&projected.nodes, "img")[0], "alt"),
        Some("bulbasaur")
    );
    assert!(find(&projected.nodes, "span")
        .iter()
        .any(|n| attr(n, "class") == Some("pokemon-name") && text(n).contains("bulbasaur")));
}

#[test]
fn grouped_missing_and_attribute_values_keep_one_cell_per_heading() {
    let source = "<catalog><product name='attribute-name'><name>Cherry</name><name>Lemon</name><stock>5</stock>row text</product><product><stock>0</stock></product></catalog>";
    let plan = render(STOCK, source, vec![]);
    let table = find(&plan.nodes, "table")[0];
    let heading_count = find(children(find(children(table), "thead")[0]), "th").len();
    let rows = find(children(find(children(table), "tbody")[0]), "tr");
    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert_eq!(
            children(row)
                .iter()
                .filter(|n| matches!(tag(n), "td" | "th"))
                .count(),
            heading_count
        );
    }
    let names = find(children(rows[0]), "td")
        .into_iter()
        .filter(|cell| text(cell).contains("Cherry"))
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 1);
    assert!(text(names[0]).contains("Lemon"));
    assert!(text(rows[0]).contains("attribute-name"));
    assert!(text(rows[0]).contains("row text"));
}

#[test]
fn missing_null_empty_and_short_urls_leave_the_name_without_an_image() {
    for field in [
        "",
        r#", "url":null"#,
        r#", "url":"""#,
        r#", "url":"/short/""#,
    ] {
        let source = format!(r#"[{{"name":"ivysaur"{field}}}]"#);
        let plan = render(pokemon_template(), &source, vec![]);
        assert!(find(&plan.nodes, "img").is_empty(), "{source}");
        assert!(text(find(&plan.nodes, "table")[0]).contains("ivysaur"));
    }
}
