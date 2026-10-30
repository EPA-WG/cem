//! Test/build-only native CEMV fixture; no JSON document or browser AST adapter.
use cem_ml::value::artifact::CemValueGraph;
pub fn source() -> String {
    let source = include_str!("passive-island.xhtml").replace("Tail</p:payload>",
        "Tail<!--first--><!--second--><!--empty--><html:section slot=\"body\"><!--nested--></html:section></p:payload>");
    let source = format!("<fixture xmlns:html=\"http://www.w3.org/1999/xhtml\">{source}<html:aside><html:mark slot=\"heading\">Referenced</html:mark><html:em slot=\"body\">Default</html:em><!--chain--></html:aside></fixture>");
    source
}
pub fn graph(bytes: &[u8]) -> CemValueGraph {
    let mut graph = CemValueGraph::decode(bytes, &Default::default()).unwrap();
    graph.roots = vec![element(&graph, "context-root")];
    let heading = element(&graph, "mark");
    let body = element(&graph, "em");
    let chain = graph
        .records
        .iter()
        .position(|n| n.kind == "comment" && n.lexical == "chain")
        .unwrap() as u32;
    for (label, targets) in [
        ("first", vec![heading, body]),
        ("second", vec![chain]),
        ("empty", vec![]),
        ("nested", vec![heading]),
        ("chain", vec![heading]),
    ] {
        let reference = graph
            .records
            .iter_mut()
            .find(|n| n.kind == "comment" && n.lexical == label)
            .unwrap();
        reference.kind = "reference".into();
        reference.lexical.clear();
        reference.targets = targets;
        reference.occurrence = label == "second";
    }
    graph.validate(&Default::default()).unwrap();
    graph
}
pub fn element(graph: &CemValueGraph, name: &str) -> u32 {
    graph
        .records
        .iter()
        .position(|n| n.kind == "element" && n.name == name)
        .unwrap() as u32
}
/// Encode deliberately invalid envelopes with a valid digest, exercising decoder
/// admission rather than failing in the fixture producer's graph validator.
pub fn unchecked(graph: &CemValueGraph) -> Vec<u8> {
    let body = rmp_serde::to_vec_named(graph).unwrap();
    let hash = cem_ml::content_cache::ContentHash::from_blake3(&body);
    let mut bytes = b"CEMV\x03".to_vec();
    for i in (0..hash.hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hash.hex[i..i + 2], 16).unwrap());
    }
    bytes.extend(body);
    bytes
}

pub fn invalid_cases(original: &CemValueGraph) -> Vec<(&'static str, CemValueGraph)> {
    let mark = element(original, "mark") as usize;
    let attr = original.records[mark].attributes[0] as usize;
    let reference = original
        .records
        .iter()
        .position(|r| r.kind == "reference")
        .unwrap();
    let mut cases = Vec::new();
    for name in [
        "foreign",
        "source",
        "opaque-source",
        "password",
        "file",
        "otp",
        "nested-credential",
        "document",
        "atomic",
        "reference-source",
        "schema",
        "cycle",
        "dangling",
        "depth",
        "expanded",
    ] {
        let mut graph = original.clone();
        match name {
            "foreign" => graph.records[mark].namespace = "urn:foreign".into(),
            "source" => graph.records[mark].provenance = None,
            "opaque-source" => {
                graph.records[mark].provenance.as_mut().unwrap().source_uri =
                    Some("data:private credential".into())
            }
            "password" | "file" | "otp" | "nested-credential" => {
                if name == "nested-credential" {
                    let section = element(&graph, "section");
                    for node in &mut graph.records {
                        if node.kind == "reference" {
                            node.targets = if node.parent == Some(section) {
                                vec![mark as u32]
                            } else {
                                vec![]
                            };
                        }
                    }
                }
                graph.records[mark].name = "input".into();
                graph.records[attr].name = if matches!(name, "password" | "file") {
                    "type"
                } else {
                    "autocomplete"
                }
                .into();
                let value = graph.records[attr].values[0] as usize;
                graph.records[value].lexical = match name {
                    "password" => "PASSWORD",
                    "file" => "file",
                    _ => "section-a ONE-TIME-CODE",
                }
                .into();
            }
            "document" => {
                graph.records[mark].kind = "document".into();
                graph.records[mark].name.clear();
                graph.records[mark].namespace.clear();
                for id in std::mem::take(&mut graph.records[mark].attributes) {
                    graph.records[id as usize].parent = None;
                }
            }
            "atomic" => {
                let mut scalar = graph.records[mark].clone();
                scalar.kind = "atomic".into();
                scalar.datatype = "string".into();
                scalar.name.clear();
                scalar.namespace.clear();
                scalar.children.clear();
                scalar.attributes.clear();
                scalar.parent = None;
                scalar.lexical = "literal".into();
                graph.records[reference].targets = vec![graph.records.len() as u32];
                graph.records.push(scalar);
            }
            "reference-source" => graph.records[reference].provenance = None,
            "schema" => {
                let payload = element(&graph, "payload") as usize;
                let schema = *graph.records[payload]
                    .attributes
                    .iter()
                    .find(|id| graph.records[**id as usize].name == "schema")
                    .unwrap() as usize;
                let value = graph.records[schema].values[0] as usize;
                graph.records[value].lexical = "urn:unsupported".into();
            }
            "cycle" => graph.records[reference].targets = vec![reference as u32],
            "dangling" => graph.records[reference].targets = vec![u32::MAX],
            "depth" | "expanded" => {
                let mut next = mark as u32;
                for _ in 0..if name == "depth" { 66 } else { 15 } {
                    let mut node = graph.records[reference].clone();
                    node.parent = None;
                    node.targets = if name == "depth" {
                        vec![next]
                    } else {
                        vec![next, next]
                    };
                    next = graph.records.len() as u32;
                    graph.records.push(node);
                }
                graph.records[reference].targets = vec![next];
            }
            _ => unreachable!(),
        }
        cases.push((name, graph));
    }
    cases
}
