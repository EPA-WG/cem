//! Imported presentation rules change cells, not the source tree or table renderer.
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    value::artifact::CemValueArtifactLimits,
};
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
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
const LIST: &str = "<catalog><pokemon><id>3</id><name>venusaur</name><type>grass</type></pokemon><pokemon><id>2</id><name>ivysaur</name><type>grass</type></pokemon></catalog>";
const PRODUCTS: &str = "<catalog><product><name>Cherry</name><stock>0</stock></product><product><name>Lemon</name><stock>5</stock></product><reference><stock>0</stock></reference></catalog>";

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
    // Scalar resolver output is host control state; source documents still enter through cem-data.
    slices.push(("images", string("https://example.test/demo/pokemon/")));
    let data = TemplateData::default()
        .with_binding("format", ItemStream::once(string("xml")))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    record(vec![("nodes", record(vec![("text", string(source))]))]),
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
    assert!(preflight.diagnostics.is_empty(), "{:?}", preflight.diagnostics);
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
fn inline_name_rule_renders_image_and_name_with_base_columns() {
    let template = pokemon_template();
    assert!(template.contains("@match='node.name == \"name\"'"));
    let plan = render(template, LIST, vec![]);
    let images = find(&plan.nodes, "img");
    assert_eq!(images.len(), 2);
    assert_eq!(
        attr(images[0], "src"),
        Some("https://example.test/demo/pokemon/3.svg")
    );
    assert_eq!(
        attr(images[1], "src"),
        Some("https://example.test/demo/pokemon/2.svg")
    );
    assert_eq!(attr(images[1], "alt"), Some("ivysaur"));
    let table = find(&plan.nodes, "table")[0];
    assert_eq!(
        find(children(find(children(table), "thead")[0]), "th")
            .iter()
            .map(|n| text(n).trim().to_owned())
            .collect::<Vec<_>>(),
        ["✓", "#text", "id", "name", "type"]
    );
    assert!(text(table).contains("ivysaur") && text(table).contains("venusaur"));
    assert!(text(table).contains("grass"));
    assert_eq!(text(find(&plan.nodes, "textarea")[0]), LIST);
}

#[test]
fn source_identity_and_name_sorting_survive_cell_overrides() {
    let initial = render(pokemon_template(), LIST, vec![]);
    let table = find(&initial.nodes, "table")[0];
    let row_heading = find(children(find(children(table), "tbody")[0]), "th")[0];
    let button = find(children(row_heading), "button")
        .into_iter()
        .find(|n| attr(n, "slice") == Some("selected"))
        .unwrap();
    let selected = attr(button, "value").unwrap();
    assert!(!selected.is_empty());
    let plan = render(
        pokemon_template(),
        LIST,
        vec![
            ("column", string("name")),
            ("mode", string("text")),
            ("selected", string(selected)),
        ],
    );
    assert_eq!(
        find(&plan.nodes, "img")
            .iter()
            .map(|n| attr(n, "alt").unwrap())
            .collect::<Vec<_>>(),
        ["ivysaur", "venusaur"]
    );
    let selected_row = find(&plan.nodes, "tr")
        .into_iter()
        .find(|n| attr(n, "aria-selected") == Some("true"))
        .unwrap();
    assert!(text(selected_row).contains("venusaur"));
}

#[test]
fn primitive_name_match_is_independent_of_path_and_other_fields_fall_back() {
    for source in [
        LIST.replace("pokemon>", "other>"),
        LIST.replace("<name>", "<p:name xmlns:p='urn:other'>")
            .replace("</name>", "</p:name>"),
    ] {
        let plan = render(pokemon_template(), &source, vec![]);
        assert_eq!(find(&plan.nodes, "img").len(), 2);
    }
    let source = LIST
        .replace("<name>", "<title>")
        .replace("</name>", "</title>");
    let plan = render(pokemon_template(), &source, vec![]);
    assert!(find(&plan.nodes, "img").is_empty());
    assert!(text(find(&plan.nodes, "table")[0]).contains("ivysaur"));
}

#[test]
fn zero_stock_rule_is_conditional_and_scoped_to_product_cells() {
    let plan = render(STOCK, PRODUCTS, vec![]);
    let warnings = find(&plan.nodes, "strong");
    assert_eq!(warnings.len(), 1);
    assert_eq!(text(warnings[0]).trim(), "Out of stock (0)");
    assert!(text(find(&plan.nodes, "table")[0]).contains('5'));
    assert_eq!(text(find(&plan.nodes, "textarea")[0]), PRODUCTS);
    for source in [
        PRODUCTS.replace("<stock>0</stock>", "<stock>7</stock>"),
        PRODUCTS.replace(
            "<stock>0</stock>",
            "<p:stock xmlns:p='urn:other'>0</p:stock>",
        ),
        PRODUCTS
            .replace("<product>", "<other>")
            .replace("</product>", "</other>"),
    ] {
        assert!(find(&render(STOCK, &source, vec![]).nodes, "strong").is_empty());
    }
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
        2
    );
}

#[test]
fn nested_collections_keep_base_structure_and_apply_the_selected_rule() {
    let source = LIST.replace("<type>grass</type>",
        "<related><entry><id>2</id><name>ivysaur</name></entry><entry><id>3</id><name>venusaur</name></entry></related>");
    let plan = render(pokemon_template(), &source, vec![]);
    assert_eq!(find(&plan.nodes, "table").len(), 3);
    assert_eq!(find(&plan.nodes, "img").len(), 6);
    let source = PRODUCTS.replace("<stock>5</stock>",
        "<stock>5</stock><suppliers><supplier><stock>0</stock></supplier><supplier><stock>0</stock></supplier></suppliers>");
    let plan = render(STOCK, &source, vec![]);
    assert_eq!(find(&plan.nodes, "table").len(), 2);
    assert_eq!(find(&plan.nodes, "strong").len(), 1);
    assert!(find(children(find(&plan.nodes, "table")[1]), "strong").is_empty());
}

#[test]
fn name_body_reuses_the_same_native_node_as_the_alt_attribute() {
    let source = LIST.replace("venusaur", "venu<em>saur</em>");
    let native = render_native(pokemon_template(), &source, vec![]);
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
        Some(AtomValue::String("name".into()))
    );
    let parent = target
        .view()
        .unwrap()
        .parent(cem_ql::eval::QueryContextScope(0))
        .unwrap()
        .unwrap();
    assert_eq!(
        parent.view().unwrap().field("name").unwrap()[0].atom(),
        Some(AtomValue::String("pokemon".into()))
    );

    let projected = render(pokemon_template(), &source, vec![]);
    let name = find(&projected.nodes, "name")[0];
    assert_eq!(text(name), "venusaur");
    assert_eq!(text(find(children(name), "em")[0]), "saur");
    assert_eq!(
        attr(find(&projected.nodes, "img")[0], "alt"),
        Some("venusaur")
    );
    assert_eq!(text(find(&projected.nodes, "textarea")[0]), source);
}

#[test]
fn grouped_missing_and_attribute_values_keep_one_cell_per_heading() {
    let source = "<catalog><pokemon name='attribute-name'><id>3</id><name>venusaur</name><name>ivysaur</name><type>grass</type>row text</pokemon><pokemon><id>2</id><type>grass</type></pokemon></catalog>";
    let plan = render(pokemon_template(), source, vec![]);
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
        .filter(|cell| text(cell).contains("venusaur"))
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 1);
    assert!(text(names[0]).contains("ivysaur"));
    assert!(text(rows[0]).contains("attribute-name"));
    assert!(text(rows[0]).contains("row text"));
    assert!(find(&plan.nodes, "img").is_empty());
}

#[test]
fn sibling_id_lookup_skips_nameless_nodes_and_takes_first_local_name_match() {
    for (source, image) in [
        ("<catalog><pokemon> \n<!--note--><id>2</id><id>3</id><name>ivysaur</name></pokemon></catalog>", true),
        ("<catalog><pokemon><p:id xmlns:p='urn:other'>2</p:id><name>ivysaur</name></pokemon></catalog>", true),
        ("<catalog><pokemon><name>ivysaur</name></pokemon></catalog>", false),
    ] {
        // A repeated row kind selects the base viewer's collection/table path.
        let source = source.replace("</catalog>", "<pokemon><type>grass</type></pokemon></catalog>");
        let plan = render(pokemon_template(), &source, vec![]);
        let images = find(&plan.nodes, "img");
        assert_eq!(images.len(), usize::from(image), "{source}");
        if image {
            assert_eq!(attr(images[0], "src"), Some("https://example.test/demo/pokemon/2.svg"));
        }
        assert!(text(find(&plan.nodes, "table")[0]).contains("ivysaur"));
    }
}
