use cem_ml::transform_config_authoring::{
    author_transform_config, GraphAuthoringCreate, GraphAuthoringEdge, GraphAuthoringEdit,
    GraphAuthoringRequest,
};

const GRAPH: &str = "// café retained\n{run | {import @id=data @src=data.xml @content-type=application/xml | {transform @id=view @src=view.xpath @template-content-type=application/vnd.cem.xpath | {export @id=result @out=result.json}}}}\n";

fn request(source: &str, edit: Option<GraphAuthoringEdit>) -> GraphAuthoringRequest {
    GraphAuthoringRequest {
        source: source.into(),
        edit,
        create: None,
        edge: None,
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
fn graph_authoring_keeps_result_identity_separate_from_template_and_exports() {
    let source = "{run | {import @id=data @src=data.xml | {transform @id=model @src=model.cemt @content-type=application/cem @schema=https://cem.dev/ns/cem-ml/1 @template-content-type=application/vnd.cem.template+cem | {export @id=service @out=result.json @content-type=application/json @schema=https://cem.dev/ns/data/json/1}}}}";
    let described = author_transform_config(request(source, None)).unwrap();
    let fields = &described
        .nodes
        .iter()
        .find(|node| node.id == "model")
        .unwrap()
        .fields;
    for (name, value) in [
        ("content-type", "application/cem"),
        ("schema", "https://cem.dev/ns/cem-ml/1"),
    ] {
        assert_eq!(
            fields
                .iter()
                .find(|field| field.id == name)
                .unwrap()
                .value
                .as_deref(),
            Some(value)
        );
    }
    assert_eq!(described.source, source);
    let edited = author_transform_config(request(
        source,
        Some(edit("model", "content-type", Some("application/xml"))),
    ))
    .unwrap();
    let model = edited.nodes.iter().find(|node| node.id == "model").unwrap();
    assert_eq!(
        model
            .fields
            .iter()
            .find(|field| field.id == "content-type")
            .unwrap()
            .value
            .as_deref(),
        Some("application/xml")
    );
    assert_eq!(
        model
            .fields
            .iter()
            .find(|field| field.id == "template-content-type")
            .unwrap()
            .value
            .as_deref(),
        Some("application/vnd.cem.template+cem")
    );
    let service = edited
        .nodes
        .iter()
        .find(|node| node.id == "service")
        .unwrap();
    assert_eq!(
        service
            .fields
            .iter()
            .find(|field| field.id == "content-type")
            .unwrap()
            .value
            .as_deref(),
        Some("application/json")
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
        edge: None,
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

fn edge(
    source: &str,
    node: &str,
    role: cem_ml::transform_config::TransformGraphEdgeRole,
    from: Option<&str>,
    name: Option<&str>,
) -> GraphAuthoringRequest {
    GraphAuthoringRequest {
        source: source.into(),
        edit: None,
        create: None,
        edge: Some(GraphAuthoringEdge {
            node: node.into(),
            role,
            source: from.map(str::to_owned),
            name: name.map(str::to_owned),
        }),
    }
}
const EDGES: &str = r#"// retained café
{run |
 {import @id=other @src="../assets/other.xml"}
 {import @id=data @src="../assets/data.xml" |
  {transform @id=view @src="../templates/view.xpath" @template-content-type=application/vnd.cem.xpath |
   {param @name=title @value="café ${literal}"}
   {export @id=result @out="../output/result.xml"}
  }
 }
}"#;

#[test]
fn graph_edges_describe_native_roles_and_preserve_exact_noops() {
    use cem_ml::transform_config::TransformGraphEdgeRole::*;
    let described = author_transform_config(request(EDGES, None)).unwrap();
    let view = described
        .nodes
        .iter()
        .find(|node| node.id == "view")
        .unwrap();
    assert!(view.named);
    assert_eq!(
        view.edges.iter().map(|field| field.id).collect::<Vec<_>>(),
        ["parent", "input", "with"]
    );
    assert_eq!(view.edges[0].source.as_deref(), Some("data"));
    assert!(!view.edges[0].can_clear);
    assert!(view.edges[2].requires_name);
    for req in [
        edge(EDGES, "view", Parent, Some("data"), None),
        edge(EDGES, "view", Input, None, None),
        edge(EDGES, "view", With, None, Some("absent")),
    ] {
        assert_eq!(author_transform_config(req).unwrap().source, EDGES);
    }
}

#[test]
fn graph_edges_override_reset_and_move_subtrees_without_changing_ids_or_resources() {
    use cem_ml::transform_config::TransformGraphEdgeRole::*;
    let before = author_transform_config(request(EDGES, None)).unwrap();
    let input = author_transform_config(edge(EDGES, "view", Input, Some("other"), None)).unwrap();
    assert_eq!(
        input
            .graph
            .nodes
            .iter()
            .find(|node| node.id == "view")
            .unwrap()
            .input_ref
            .as_deref(),
        Some("other")
    );
    assert!(!input
        .graph
        .edges
        .iter()
        .any(|edge| edge.to == "view" && edge.role == Parent));
    let reset = author_transform_config(edge(&input.source, "view", Input, None, None)).unwrap();
    assert_eq!(reset.graph.nodes, before.graph.nodes);
    assert_eq!(reset.graph.edges, before.graph.edges);
    let moved = author_transform_config(edge(EDGES, "view", Parent, Some("other"), None)).unwrap();
    for node in &before.graph.nodes {
        assert!(moved.graph.nodes.contains(node));
    }
    assert!(moved
        .graph
        .edges
        .iter()
        .any(|edge| edge.from == "other" && edge.to == "view" && edge.role == Parent));
    assert!(moved
        .graph
        .edges
        .iter()
        .any(|edge| edge.from == "view" && edge.to == "result" && edge.role == Parent));
    let overridden_move =
        author_transform_config(edge(&input.source, "view", Parent, Some("other"), None)).unwrap();
    assert_eq!(overridden_move.graph.edges, input.graph.edges);
    let reordered = author_transform_config(edge(EDGES, "data", Parent, None, None)).unwrap();
    assert_eq!(reordered.source, EDGES);
}

#[test]
fn graph_edges_add_replace_remove_named_inputs_and_preserve_duplicate_source_edges() {
    use cem_ml::transform_config::TransformGraphEdgeRole::*;
    let first =
        author_transform_config(edge(EDGES, "view", With, Some("other"), Some("palette"))).unwrap();
    let second = author_transform_config(edge(
        &first.source,
        "view",
        With,
        Some("other"),
        Some("stats"),
    ))
    .unwrap();
    assert_eq!(
        second
            .graph
            .edges
            .iter()
            .filter(|edge| edge.from == "other" && edge.to == "view" && edge.role == With)
            .count(),
        2
    );
    let removed =
        author_transform_config(edge(&second.source, "view", With, None, Some("palette"))).unwrap();
    assert_eq!(
        removed
            .graph
            .edges
            .iter()
            .filter(|edge| edge.from == "other" && edge.to == "view" && edge.role == With)
            .count(),
        1
    );
    let changed = author_transform_config(edge(
        &removed.source,
        "view",
        With,
        Some("data"),
        Some("stats"),
    ))
    .unwrap();
    let view = changed
        .graph
        .nodes
        .iter()
        .find(|node| node.id == "view")
        .unwrap();
    assert_eq!(view.with.get("stats").map(String::as_str), Some("data"));
    assert_eq!(
        view.params.get("title").map(String::as_str),
        Some("café ${literal}")
    );
    assert_eq!(
        author_transform_config(edge(
            &changed.source,
            "view",
            With,
            Some("data"),
            Some("stats")
        ))
        .unwrap()
        .source,
        changed.source
    );
}

#[test]
fn graph_edges_reject_cycles_unknown_nodes_invalid_roles_names_and_ambiguous_edits() {
    use cem_ml::transform_config::TransformGraphEdgeRole::*;
    for req in [
        edge(EDGES, "view", Parent, Some("view"), None),
        edge(EDGES, "view", Parent, Some("result"), None),
        edge(EDGES, "view", Parent, None, None),
        edge(EDGES, "other", Parent, Some("data"), None),
        edge(EDGES, "view", Input, Some("view"), None),
        edge(EDGES, "view", Input, Some("result"), None),
        edge(EDGES, "view", Input, Some("unknown"), None),
        edge(EDGES, "result", Input, Some("data"), None),
        edge(EDGES, "view", With, Some("view"), Some("x")),
        edge(EDGES, "view", With, Some("result"), Some("x")),
        edge(EDGES, "view", With, Some("other"), Some("bad:name")),
        edge(EDGES, "view", With, Some("other"), None),
        edge(EDGES, "view", Input, Some("other"), Some("unexpected")),
        edge(EDGES, "missing", Input, Some("other"), None),
    ] {
        assert!(author_transform_config(req).is_err());
    }
    assert!(
        author_transform_config(edge(EDGES, "view", Input, Some("view"), None))
            .unwrap_err()
            .contains("cycle")
    );
    assert!(
        author_transform_config(edge(EDGES, "view", Parent, Some("view"), None))
            .unwrap_err()
            .contains("cycle")
    );
    let mut ambiguous = edge(EDGES, "view", Input, Some("other"), None);
    ambiguous.edit = Some(edit("result", "out", Some("x")));
    assert!(author_transform_config(ambiguous).is_err());
    let required = "{run | {import @id=a @src=a.cem | {join @id=j @mode=zip @with:b=b | {export @id=out @out=x}}} {import @id=b @src=b.cem}}";
    assert!(author_transform_config(edge(required, "j", With, None, Some("b"))).is_err());
    let opaque = "{run | {import @id=a @src=a.svg @opaque=true @content-type=image/svg+xml | {export @id=out @out=a.svg}} {import @id=b @src=b.svg @opaque=true @content-type=image/svg+xml | {export @id=other @out=b.svg}}}";
    assert!(
        author_transform_config(edge(opaque, "out", Parent, Some("b"), None))
            .unwrap_err()
            .contains("opaque_direct_export_required")
    );
    let mixed = "{run | {import @id=a @src=a.xml | {convert @id=c @content-type=application/xml}} {import @id=b @src=b.xml | {export @out=x}}}";
    assert!(author_transform_config(edge(mixed, "c", Parent, Some("b"), None)).is_err());
}
