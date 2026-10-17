use cem_ml::transform_config_authoring::{
    author_transform_config, GraphAuthoringEdit, GraphAuthoringRequest,
};

const GRAPH: &str = "// café retained\n{run | {import @id=data @src=data.xml @content-type=application/xml | {transform @id=view @src=view.xpath @template-content-type=application/vnd.cem.xpath | {export @id=result @out=result.json}}}}\n";

fn request(source: &str, edit: Option<GraphAuthoringEdit>) -> GraphAuthoringRequest {
    GraphAuthoringRequest {
        source: source.into(),
        edit,
    }
}
fn edit(node: &str, attribute: &str, value: Option<&str>) -> GraphAuthoringEdit {
    GraphAuthoringEdit {
        node: node.into(),
        attribute: attribute.into(),
        value: value.map(str::to_owned),
    }
}

#[test]
fn graph_authoring_describes_native_nodes_and_retains_noop_bytes() {
    let result = author_transform_config(request(GRAPH, None)).unwrap();
    assert_eq!(result.source, GRAPH);
    assert_eq!(
        result
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>(),
        ["data", "view", "result"]
    );
    let field = result.nodes[0]
        .fields
        .iter()
        .find(|field| field.id == "src")
        .unwrap();
    assert!(field.required);
    assert_eq!(field.value.as_deref(), Some("data.xml"));
    assert_eq!(
        author_transform_config(request(GRAPH, Some(edit("data", "src", Some("data.xml")))))
            .unwrap()
            .source,
        GRAPH
    );
}

#[test]
fn graph_authoring_edits_native_ast_and_validates_candidate_before_publication() {
    let before = author_transform_config(request(GRAPH, None)).unwrap();
    let changed = author_transform_config(request(
        GRAPH,
        Some(edit(
            "result",
            "out",
            Some("café \\\"{literal} $(text).json"),
        )),
    ))
    .unwrap();
    assert_eq!(
        changed.nodes[2]
            .fields
            .iter()
            .find(|field| field.id == "out")
            .unwrap()
            .value
            .as_deref(),
        Some("café \\\"{literal} $(text).json")
    );
    assert_eq!(changed.graph.edges, before.graph.edges);
    assert_eq!(changed.graph.nodes[0], before.graph.nodes[0]);
    assert_eq!(changed.graph.nodes[1], before.graph.nodes[1]);
    let with_optional = author_transform_config(request(
        GRAPH,
        Some(edit("result", "style-policy", Some("inline"))),
    ))
    .unwrap();
    let reset = author_transform_config(request(
        &with_optional.source,
        Some(edit("result", "style-policy", None)),
    ))
    .unwrap();
    assert!(reset.nodes[2]
        .fields
        .iter()
        .find(|field| field.id == "style-policy")
        .unwrap()
        .value
        .is_none());
    for change in [
        edit("data", "src", None),
        edit("data", "id", Some("renamed")),
        edit("missing", "out", Some("x")),
        edit("result", "style-policy", Some("invented")),
        edit("view", "input", Some("#missing")),
        edit("view", "input", Some("#view")),
    ] {
        assert!(author_transform_config(request(GRAPH, Some(change))).is_err());
    }
}

#[test]
fn graph_authoring_fails_closed_for_invalid_ambiguous_or_oversized_sources() {
    for source in [
        "{run | {import @src=x @opaque=maybe}}",
        "{run | {import @id=data @src=x @src=y}}",
        "{run | {import @src=x @id=data | {export @id=data @out=y}}}",
    ] {
        assert!(
            author_transform_config(request(source, None)).is_err(),
            "{source}"
        );
    }
    assert!(author_transform_config(request(&" ".repeat(1024 * 1024 + 1), None)).is_err());
}
