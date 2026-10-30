use super::*;
use crate::eval::{
    portable::{decode_values, encode_values},
    QueryContextScope,
};
use cem_ml::value::artifact::CemValueGraph;
#[path = "../../fixtures/component-application/payload_references.rs"]
mod fixture;
fn graph() -> CemValueGraph {
    let tree = cem_ml::import::import_data_bytes(
        fixture::source().as_bytes(),
        "application/xml",
        "cem",
        "app://island/references.xhtml",
    )
    .unwrap();
    let bytes = encode_values(
        &ItemStream::once(crate::eval::imported_cem_tree(tree)),
        &Default::default(),
    )
    .unwrap();
    fixture::graph(&bytes)
}
fn bind_graph(graph: &CemValueGraph) -> Result<TemplateData, String> {
    bind(
        &fixture::unchecked(graph),
        "island-one",
        &super::super::render_control().unwrap(),
    )
}
fn frame(data: &TemplateData) -> &BTreeMap<String, Vec<Item>> {
    let [Item::Record(fields)] = data.bindings["datadom"].items.as_slice() else {
        panic!()
    };
    fields
}
#[test]
fn references_keep_payload_occurrences_and_source_parents_but_index_rendered_slot_roots() {
    let graph = graph();
    let data = bind_graph(&graph).unwrap();
    let frame = frame(&data);
    let [Item::Record(slots)] = frame["slots"].as_slice() else {
        panic!()
    };
    assert_eq!(slots["heading"].len(), 4);
    assert_eq!(slots["body"].len(), 2);
    assert_eq!(slots[""].len(), 1);
    let payload = children(&frame["payload"][0]).unwrap();
    assert_eq!(field(&payload[3], "kind").as_deref(), Some("reference"));
    let targets = payload[3].view().unwrap().field("targets").unwrap();
    assert_eq!(targets[0].identity(), slots["heading"][2].identity());
    assert_eq!(targets[0].identity(), slots["heading"][3].identity());
    assert_eq!(targets[1].identity(), slots["body"][0].identity());
    let parent = targets[0]
        .view()
        .unwrap()
        .parent(QueryContextScope(0))
        .unwrap()
        .unwrap();
    assert_eq!(field(&parent, "name").as_deref(), Some("aside"));
    let origin = &graph.records[fixture::element(&graph, "mark") as usize];
    assert_eq!(targets[0].source_map(), Some(origin.source.clone()));
    assert_eq!(
        targets[0].view().unwrap().provenance().unwrap().source_key,
        origin.provenance.as_ref().unwrap().source_key
    );
    // A nested reference stays inside its element and does not enter a root slot.
    let nested = children(&slots["body"][1]).unwrap();
    assert_eq!(field(&nested[0], "kind").as_deref(), Some("reference"));
    let encoded = encode_values(
        &ItemStream::from_items(slots["heading"].clone()),
        &Default::default(),
    )
    .unwrap();
    let round = decode_values(&encoded, &Default::default()).unwrap();
    assert_eq!(round.items[2].identity(), round.items[3].identity());
    assert_eq!(
        field(
            &round.items[2]
                .view()
                .unwrap()
                .parent(QueryContextScope(0))
                .unwrap()
                .unwrap(),
            "name"
        )
        .as_deref(),
        Some("aside")
    );
}
#[test]
fn reference_targets_cannot_bypass_source_namespace_schema_or_credential_policy() {
    let original = graph();
    let heading = fixture::element(&original, "mark") as usize;
    let attr = original.records[heading].attributes[0] as usize;
    for mutation in 0..9 {
        let mut graph = original.clone();
        match mutation {
            0 => graph.records[heading].namespace = "urn:foreign".into(),
            1 => graph.records[heading].provenance = None,
            2 => graph.records[heading].source = Default::default(),
            3 | 4 | 5 => {
                graph.records[heading].name = "input".into();
                graph.records[attr].name = if mutation == 3 {
                    "type"
                } else {
                    "autocomplete"
                }
                .into();
                let value = graph.records[attr].values[0] as usize;
                graph.records[value].lexical = match mutation {
                    3 => "PASSWORD",
                    4 => "section-a ONE-TIME-CODE",
                    _ => "current-password",
                }
                .into();
            }
            6 => graph.records[heading].kind = "document".into(),
            7 => {
                graph
                    .records
                    .iter_mut()
                    .find(|n| n.kind == "reference")
                    .unwrap()
                    .provenance = None
            }
            _ => {
                let payload = fixture::element(&graph, "payload") as usize;
                let schema = *graph.records[payload]
                    .attributes
                    .iter()
                    .find(|id| graph.records[**id as usize].name == "schema")
                    .unwrap() as usize;
                let value = graph.records[schema].values[0] as usize;
                graph.records[value].lexical = "urn:unsupported".into();
            }
        }
        assert_eq!(
            bind_graph(&graph).unwrap_err(),
            INVALID,
            "mutation {mutation}"
        );
    }
    assert!(bind_graph(&original).is_ok());
}
#[test]
fn cyclic_dangling_deep_and_exponentially_expanded_graphs_refuse_before_binding() {
    let original = graph();
    for mutation in 0..4 {
        let mut graph = original.clone();
        let reference = graph
            .records
            .iter()
            .position(|n| n.kind == "reference")
            .unwrap();
        match mutation {
            0 => graph.records[reference].targets = vec![reference as u32],
            1 => graph.records[reference].targets = vec![u32::MAX],
            _ => {
                let target = graph.records[reference].targets[0];
                let mut next = target;
                for _ in 0..if mutation == 2 { 66 } else { 15 } {
                    let mut item = graph.records[reference].clone();
                    item.parent = None;
                    item.targets = if mutation == 2 {
                        vec![next]
                    } else {
                        vec![next, next]
                    };
                    next = graph.records.len() as u32;
                    graph.records.push(item);
                }
                graph.records[reference].targets = vec![next];
            }
        }
        assert_eq!(bind_graph(&graph).unwrap_err(), INVALID);
    }
    assert!(bind_graph(&original).is_ok());
}

#[test]
fn native_render_and_export_keep_reference_targets_without_clones() {
    use crate::{
        application_runtime::owners::CompiledDeclarationOwner,
        declaration_artifact::build_declaration_artifact,
        template_artifact::TemplateArtifactSourceMapMode,
    };
    let graph = graph();
    let input = graph.encode(&Default::default()).unwrap();
    let source = r#"<cem-element tag="cem-owned-references"><template id="cem-owned-references" type="text/cem-ml">{article @id='{$instanceID}' | {header | {$datadom.slots.heading}}{main | {$datadom.slots.body}}}</template></cem-element>"#;
    let built = build_declaration_artifact(
        source,
        "https://app.test/components/references.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap();
    let owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    let html = owner.render_island_native(&input, "island-one").unwrap();
    assert_eq!(html.matches("Referenced").count(), 3, "{html}");
    assert_eq!(html.matches("Default").count(), 1);
    assert!(html.contains("Heading"));
    assert!(!html.contains("Tail"));
    let bytes = owner.project_island_native(&input, "island-one").unwrap();
    let output = CemValueGraph::decode(&bytes, &Default::default()).unwrap();
    let marks = output
        .records
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == "element" && n.name == "mark")
        .collect::<Vec<_>>();
    assert_eq!(
        marks.len(),
        1,
        "repetitions must share one original source node"
    );
    let (id, mark) = marks[0];
    let original = &graph.records[fixture::element(&graph, "mark") as usize];
    assert_eq!(mark.source, original.source);
    assert_eq!(
        mark.provenance.as_ref().unwrap().source_key,
        original.provenance.as_ref().unwrap().source_key
    );
    assert_eq!(output.records[mark.parent.unwrap() as usize].name, "aside");
    assert!(
        output
            .records
            .iter()
            .filter(|r| r.kind == "reference" && r.targets.contains(&(id as u32)))
            .count()
            >= 3
    );
    assert!(owner.render_island_native(&bytes, "island-one").is_err());
    assert_eq!(
        owner.render_island_native(&input, "island-one").unwrap(),
        html
    );
}

#[test]
fn nested_only_reference_targets_and_empty_reference_sources_still_require_proof() {
    let mut graph = graph();
    let target = fixture::element(&graph, "mark");
    let section = fixture::element(&graph, "section");
    for node in &mut graph.records {
        if node.kind == "reference" {
            node.targets = if node.parent == Some(section) {
                vec![target]
            } else {
                vec![]
            };
        }
    }
    assert!(bind_graph(&graph).is_ok());
    let mut empty = graph.clone();
    let payload = fixture::element(&empty, "payload");
    let reference = empty
        .records
        .iter_mut()
        .find(|r| r.kind == "reference" && r.targets.is_empty() && r.parent == Some(payload))
        .unwrap();
    reference.provenance = None;
    assert_eq!(bind_graph(&empty).unwrap_err(), INVALID);
    let attr = graph.records[target as usize].attributes[0] as usize;
    graph.records[attr].name = "autocomplete".into();
    let value = graph.records[attr].values[0] as usize;
    graph.records[value].lexical = "new-password".into();
    assert_eq!(bind_graph(&graph).unwrap_err(), INVALID);
}

#[test]
fn native_wasm_fixture_refusals_cover_valid_and_malformed_reference_graphs() {
    let graph = graph();
    for (name, invalid) in fixture::invalid_cases(&graph) {
        // All semantic cases also pass the ordinary graph contract: refusal
        // cannot accidentally come from a malformed fixture producer.
        if !matches!(name, "cycle" | "dangling" | "depth" | "expanded") {
            invalid.validate(&Default::default()).unwrap();
        }
        assert_eq!(bind_graph(&invalid).unwrap_err(), INVALID, "{name}");
    }
    assert!(bind_graph(&graph).is_ok());
}
