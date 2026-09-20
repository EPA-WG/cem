//! XML-VIEW-4: authored table controls over the shared retained CEM tree.
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
    render::{
        compile_template, render_compiled_template, CompileTemplateOptions, RenderPlan,
        RenderPlanNode, TemplateData,
    },
};

const VIEW: &str = include_str!("../../cem-elements/demo/data-table-view.cemt");
const FRUIT: &str = "<r>\n<row qty='2'>Cherry</row>\n<row qty='10'>Lemon</row>\n<row qty='02'>Apple</row>\n<row>Banana</row>\n</r>";

fn text(value: &str) -> Item {
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
fn render(source: &str, format: &str, slices: Vec<(String, String)>, revision: i64) -> RenderPlan {
    let slices = Item::Record(
        slices
            .into_iter()
            .map(|(key, value)| (key, vec![text(&value)]))
            .collect(),
    );
    let data = TemplateData::default()
        .with_binding("format", ItemStream::once(text(format)))
        .with_binding("inspector", ItemStream::once(text("true")))
        .with_binding(
            "datadom",
            ItemStream::once(record(vec![
                (
                    "payload",
                    record(vec![("nodes", record(vec![("text", text(source))]))]),
                ),
                ("slices", slices),
                ("attributes", record(vec![("inspector", text("true"))])),
                (
                    "eventPayloads",
                    record(vec![(
                        "source",
                        record(vec![(
                            "revision",
                            Item::Atomic(AtomValue::Integer(revision)),
                        )]),
                    )]),
                ),
            ])),
        );
    let artifact = compile_template(
        VIEW,
        &CompileTemplateOptions {
            host_bindings: data.bindings.keys().cloned().collect(),
            ..Default::default()
        },
    );
    let plan = render_compiled_template(&artifact, &data);
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    plan
}
fn tag(node: &RenderPlanNode) -> &str {
    if let RenderPlanNode::Element { tag, .. } = node {
        tag
    } else {
        ""
    }
}
fn children(node: &RenderPlanNode) -> &[RenderPlanNode] {
    if let RenderPlanNode::Element { children, .. } = node {
        children
    } else {
        &[]
    }
}
fn attr<'a>(node: &'a RenderPlanNode, name: &str) -> Option<&'a str> {
    if let RenderPlanNode::Element { attributes, .. } = node {
        attributes
            .iter()
            .find(|attr| attr.name == name)
            .map(|attr| attr.value.as_str())
    } else {
        None
    }
}
fn content(node: &RenderPlanNode) -> String {
    if let RenderPlanNode::Text { text, .. } = node {
        text.clone()
    } else {
        children(node).iter().map(content).collect()
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
fn rows(table: &RenderPlanNode) -> Vec<&RenderPlanNode> {
    let body = children(table)
        .iter()
        .find(|node| tag(node) == "tbody")
        .unwrap();
    children(body)
        .iter()
        .filter(|node| tag(node) == "tr")
        .collect()
}
fn cells(row: &RenderPlanNode) -> Vec<String> {
    children(row)
        .iter()
        .filter(|node| tag(node) == "td")
        .map(|node| content(node).trim().into())
        .collect()
}
fn row_values(table: &RenderPlanNode) -> Vec<String> {
    rows(table)
        .into_iter()
        .map(|row| cells(row).last().unwrap().clone())
        .collect()
}
fn checkbox(row: &RenderPlanNode) -> &RenderPlanNode {
    find(children(row), "input")[0]
}
fn selected(table: &RenderPlanNode) -> Vec<String> {
    rows(table)
        .into_iter()
        .filter(|row| attr(checkbox(row), "checked").is_some())
        .map(|row| cells(row).last().unwrap().clone())
        .collect()
}
fn state(table: &RenderPlanNode, value: &str) -> (String, String) {
    let button = find(children(table), "button")
        .into_iter()
        .find(|node| attr(node, "value") == Some(value))
        .unwrap();
    (attr(button, "slice").unwrap().into(), value.into())
}

#[test]
fn inspector_controls_use_common_nodes_in_all_four_formats() {
    for (format, source) in [
        ("xml", "<r><row qty='2'>🍒</row><row qty='1'>🍋</row></r>"),
        ("csv", "qty,fruit\n2,🍒\n1,🍋\n"),
        ("yaml", "- qty: 2\n  fruit: 🍒\n- qty: 1\n  fruit: 🍋"),
        ("json", r#"[{"qty":2,"fruit":"🍒"},{"qty":1,"fruit":"🍋"}]"#),
    ] {
        let plan = render(source, format, vec![], 0);
        let table = find(&plan.nodes, "table")[0];
        assert_eq!(find(children(table), "caption").len(), 1, "{format}");
        assert_eq!(rows(table).len(), 2);
        assert!(row_values(table).iter().any(|value| value.contains('🍒')));
        for row in rows(table) {
            assert!(attr(checkbox(row), "slice")
                .unwrap()
                .starts_with("row.cem-source:1:"));
            assert!(attr(checkbox(row), "checked").is_none());
        }
        assert!(find(children(table), "th")
            .iter()
            .all(|node| attr(node, "scope") == Some("col")));
    }
}

#[test]
fn heterogeneous_columns_and_text_only_rows_remain_visible() {
    let plan = render("<r xmlns:a='urn:one' xmlns:b='urn:one'><a:row same=''>pre<![CDATA[🍒]]><same>child</same>post</a:row><b:row later=''>next</b:row></r>", "xml", vec![], 0);
    let table = find(&plan.nodes, "table")[0];
    let headings = find(children(table), "th");
    let names: Vec<_> = headings
        .iter()
        .skip(1)
        .map(|node| content(node).replace(['↑', '↓'], "").trim().to_owned())
        .collect();
    assert_eq!(names, ["@same", "#text", "same", "@later"]);
    assert!(!content(table).contains("xmlns"));
    let first = cells(rows(table)[0]);
    assert_eq!(&first[1..3], &["\"\"", "pre🍒post"]);
    assert_eq!(first.last().unwrap(), "∅");
    let plan = render(
        "<r><name>ivysaur</name><name>venusaur</name></r>",
        "xml",
        vec![],
        0,
    );
    assert_eq!(
        row_values(find(&plan.nodes, "table")[0]),
        ["ivysaur", "venusaur"]
    );
}

#[test]
fn multiple_selections_follow_source_keys_through_stable_sorting() {
    let initial = render(FRUIT, "xml", vec![], 0);
    let table = find(&initial.nodes, "table")[0];
    let selected_slices: Vec<_> = [1, 2]
        .map(|index| {
            (
                attr(checkbox(rows(table)[index]), "slice").unwrap().into(),
                "edit-0".into(),
            )
        })
        .into();
    let mode = attr(find(children(table), "select")[0], "slice")
        .unwrap()
        .to_owned();
    for (sort, number, expected) in [
        (
            "ascending:@qty",
            false,
            vec!["Apple", "Lemon", "Cherry", "Banana"],
        ),
        (
            "ascending:@qty",
            true,
            vec!["Cherry", "Apple", "Lemon", "Banana"],
        ),
        (
            "descending:@qty",
            true,
            vec!["Lemon", "Cherry", "Apple", "Banana"],
        ),
    ] {
        let mut slices = selected_slices.clone();
        slices.push(state(table, sort));
        slices.push((mode.clone(), if number { "number" } else { "text" }.into()));
        let sorted = render(FRUIT, "xml", slices, 0);
        let table = find(&sorted.nodes, "table")[0];
        assert_eq!(row_values(table), expected);
        let mut selected = selected(table);
        selected.sort();
        assert_eq!(selected, ["Apple", "Lemon"]);
        let active: Vec<_> = find(children(table), "th")
            .into_iter()
            .filter(|node| attr(node, "aria-sort").is_some())
            .collect();
        assert_eq!(active.len(), 1);
        assert_eq!(
            attr(active[0], "aria-sort"),
            Some(sort.split(':').next().unwrap())
        );
    }
}

#[test]
fn source_edits_and_same_source_reloads_clear_selection() {
    let initial = render(FRUIT, "xml", vec![], 0);
    let table = find(&initial.nodes, "table")[0];
    let selection = (
        attr(checkbox(rows(table)[0]), "slice").unwrap().into(),
        "edit-0".into(),
    );
    for (source, revision) in [
        (FRUIT.to_owned(), 1),
        (FRUIT.replace("Cherry", "Changed"), 1),
        (FRUIT.to_owned(), 2),
    ] {
        let plan = render(&source, "xml", vec![selection.clone()], revision);
        assert!(selected(find(&plan.nodes, "table")[0]).is_empty());
    }
    let invalid = render("<r>", "xml", vec![selection], 3);
    assert!(find(&invalid.nodes, "table").is_empty());
    assert!(find(&invalid.nodes, "p")
        .iter()
        .any(|node| attr(node, "role") == Some("alert")));
}

#[test]
fn nested_tables_keep_sort_and_selection_independent() {
    let source = "<r><row><group><v>red</v><v>green</v></group></row><row><group><v>yellow</v><v>blue</v></group></row></r>";
    let initial = render(source, "xml", vec![], 0);
    let tables = find(&initial.nodes, "table");
    assert_eq!(tables.len(), 3);
    let choice = (
        attr(checkbox(rows(tables[1])[0]), "slice").unwrap().into(),
        "edit-0".into(),
    );
    let sorted = render(
        source,
        "xml",
        vec![choice, state(tables[1], "ascending:#text")],
        0,
    );
    let tables = find(&sorted.nodes, "table");
    assert_eq!(row_values(tables[1]), ["green", "red"]);
    assert_eq!(row_values(tables[2]), ["yellow", "blue"]);
    assert_eq!(selected(tables[1]), ["red"]);
    assert!(selected(tables[2]).is_empty());
}
