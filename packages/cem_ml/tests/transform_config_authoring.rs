use cem_ml::transform_config_authoring::{
    author_transform_config, GraphAuthoringCreate, GraphAuthoringEdit, GraphAuthoringRequest,
};

const GRAPH: &str = "// café retained\n{run | {import @id=data @src=data.xml @content-type=application/xml | {transform @id=view @src=view.xpath @template-content-type=application/vnd.cem.xpath | {export @id=result @out=result.json}}}}\n";

fn request(source: &str, edit: Option<GraphAuthoringEdit>) -> GraphAuthoringRequest {
    GraphAuthoringRequest {
        source: source.into(),
        edit,
        create: None,
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

fn create(
    source: &str,
    parent: Option<&str>,
    kind: &str,
    id: &str,
    attributes: &[(&str, &str)],
) -> GraphAuthoringRequest {
    GraphAuthoringRequest {
        source: source.into(),
        edit: None,
        create: Some(GraphAuthoringCreate {
            parent: parent.map(str::to_owned),
            kind: kind.into(),
            id: id.into(),
            attributes: attributes
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into()))
                .collect(),
        }),
    }
}

#[test]
fn graph_creation_describes_schema_types_and_only_authored_parent_choices() {
    let described = author_transform_config(request(GRAPH, None)).unwrap();
    let root = &described.parents[0];
    assert_eq!(root.id, "");
    assert_eq!(root.kind, "run");
    assert_eq!(root.children, ["import"]);
    assert!(described.parents.iter().all(|parent| parent.id != "result"));
    assert_eq!(described.node_types.len(), 6);
    let import = described
        .node_types
        .iter()
        .find(|kind| kind.id == "import")
        .unwrap();
    assert!(import
        .fields
        .iter()
        .any(|field| field.id == "src" && field.required));
    assert!(described.node_types.iter().all(|kind| kind
        .fields
        .iter()
        .all(|field| field.id != "id"
            && !field.id.contains(':')
            && !field.id.chars().any(char::is_uppercase))));
    let empty = author_transform_config(request("{run}", None)).unwrap();
    assert_eq!(empty.parents.len(), 1);
}

#[test]
fn graph_creation_appends_every_native_kind_and_preserves_existing_nodes_edges_and_literals() {
    for (parent, kind, attributes) in [
        (
            None,
            "import",
            vec![
                ("src", "data café.xml"),
                ("content-type", "application/xml"),
            ],
        ),
        (Some("data"), "join", vec![("mode", "collect")]),
        (
            Some("data"),
            "convert",
            vec![("content-type", "application/json")],
        ),
        (
            Some("data"),
            "transform",
            vec![
                ("src", "branch.xpath"),
                ("template-content-type", "application/vnd.cem.xpath"),
            ],
        ),
        (
            Some("data"),
            "rewrite-importmap",
            vec![("target-map", "map.json")],
        ),
        (
            Some("view"),
            "export",
            vec![("out", r#"café \"{literal} $(text).json"#)],
        ),
    ] {
        let before = author_transform_config(request(GRAPH, None)).unwrap();
        let created =
            author_transform_config(create(GRAPH, parent, kind, "new-branch", &attributes))
                .unwrap();
        assert_eq!(created.graph.nodes.len(), before.graph.nodes.len() + 1);
        for node in &before.graph.nodes {
            assert!(created.graph.nodes.contains(node), "{kind}: {node:?}");
        }
        for edge in &before.graph.edges {
            assert!(created.graph.edges.contains(edge));
        }
        assert_eq!(
            created.graph.edges.len(),
            before.graph.edges.len() + usize::from(parent.is_some())
        );
        let node = created
            .nodes
            .iter()
            .find(|node| node.id == "new-branch")
            .unwrap();
        for (name, value) in attributes {
            assert_eq!(
                node.fields
                    .iter()
                    .find(|field| field.id == name)
                    .unwrap()
                    .value
                    .as_deref(),
                Some(value)
            );
        }
        assert_eq!(
            author_transform_config(request(&created.source, None))
                .unwrap()
                .source,
            created.source
        );
    }
    let new_root = author_transform_config(create(
        "{run}",
        None,
        "import",
        "data",
        &[("src", "data.xml")],
    ))
    .unwrap();
    assert_eq!(new_root.graph.nodes[0].id, "data");
}

#[test]
fn graph_creation_rejects_ambiguous_invalid_and_unpreservable_candidates() {
    for candidate in [
        create(GRAPH, None, "export", "new", &[("out", "x")]),
        create(GRAPH, Some("result"), "export", "new", &[("out", "x")]),
        create(GRAPH, Some("absent"), "export", "new", &[("out", "x")]),
        create(
            GRAPH,
            Some("data"),
            "param",
            "new",
            &[("name", "x"), ("value", "x")],
        ),
        create(GRAPH, Some("data"), "export", "result", &[("out", "x")]),
        create(GRAPH, Some("data"), "export", "", &[("out", "x")]),
        create(GRAPH, Some("data"), "export", " padded ", &[("out", "x")]),
        create(
            GRAPH,
            Some("data"),
            "export",
            &"x".repeat(257),
            &[("out", "x")],
        ),
        create(
            GRAPH,
            Some("data"),
            "export",
            "huge",
            &[("out", &"x".repeat(1024 * 1024 + 1))],
        ),
        create(GRAPH, Some("data"), "export", "new", &[]),
        create(
            GRAPH,
            Some("data"),
            "export",
            "new",
            &[("out", "x"), ("id", "other")],
        ),
        create(
            GRAPH,
            Some("data"),
            "export",
            "new",
            &[("out", "x"), ("stylePolicy", "inline")],
        ),
        create(GRAPH, Some("data"), "join", "new", &[("mode", "invented")]),
        create(
            GRAPH,
            Some("data"),
            "convert",
            "new",
            &[("input", "absent")],
        ),
        create(
            GRAPH,
            Some("data"),
            "transform",
            "new",
            &[("src", "x.xpath"), ("input", "new")],
        ),
        create(
            GRAPH,
            Some("data"),
            "export",
            "new",
            &[("out", "both\"and'quotes")],
        ),
    ] {
        assert!(author_transform_config(candidate).is_err());
    }
    let mut both = create(GRAPH, Some("data"), "export", "new", &[("out", "x")]);
    both.edit = Some(edit("result", "out", Some("x")));
    assert!(author_transform_config(both).is_err());
    let mixed = "{run | {import @id=data @src=x | {export @out=x}} {import @src=y}}";
    assert!(author_transform_config(create(
        mixed,
        Some("data"),
        "export",
        "new",
        &[("out", "z")]
    ))
    .is_err());
    let hundred = format!(
        "{{run | {} }}",
        (0..100)
            .map(|index| format!("{{import @id=i{index} @src=x}} "))
            .collect::<String>()
    );
    assert!(author_transform_config(create(
        &hundred,
        None,
        "import",
        "overflow",
        &[("src", "x")]
    ))
    .is_err());
}

#[test]
fn graph_creation_retains_existing_parameter_bindings_and_resource_references() {
    let source = r#"{run |
        {import @id=palette @src="../assets/palette.xml"}
        {import @id=data @src="../assets/data.xml" |
            {transform @id=view @src="../templates/view.xpath" @template-content-type=application/vnd.cem.xpath @with:palette=palette |
                {param @name=label @value="café ${literal}"}
                {export @id=result @out="../output/result.xml"}
            }
        }
    }"#;
    let before = author_transform_config(request(source, None)).unwrap();
    let after = author_transform_config(create(
        source,
        Some("data"),
        "export",
        "copy",
        &[("out", "../output/copy.xml")],
    ))
    .unwrap();
    for node in &before.graph.nodes {
        assert!(after.graph.nodes.contains(node));
    }
    for edge in &before.graph.edges {
        assert!(after.graph.edges.contains(edge));
    }
    let transform = after
        .graph
        .nodes
        .iter()
        .find(|node| node.id == "view")
        .unwrap();
    assert_eq!(
        transform.params.get("label").map(String::as_str),
        Some("café ${literal}")
    );
    assert_eq!(
        transform.with.get("palette").map(String::as_str),
        Some("palette")
    );
}
