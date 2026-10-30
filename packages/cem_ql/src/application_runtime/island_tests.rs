use super::*;
use crate::eval::{
    imported_cem_tree,
    portable::{decode_values, encode_values},
};
pub const SOURCE: &str = include_str!("../../fixtures/component-application/passive-island.xhtml");
fn capsule(source: &str) -> Vec<u8> {
    let tree = cem_ml::import::import_data_bytes(
        source.as_bytes(),
        "application/xml",
        "cem",
        "app://island/source.xhtml",
    )
    .unwrap();
    encode_values(
        &ItemStream::once(imported_cem_tree(tree)),
        &Default::default(),
    )
    .unwrap()
}
#[test]
fn qualified_topology_tracks_canonical_schema_and_lifecycle() {
    use cem_ml::{
        source::{BytesSource, SourceId},
        tokenizer::{cem::CemTokenizer, SchemaTokenKind, SchemaTokenizer},
    };
    let schema = include_str!(
        "../../../cem_ml/schema-packages/cem-data-island/v1/schema/cem-data-island.cem"
    );
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), schema.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    let mut namespaces = Vec::new();
    let mut declared = None;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(&token.kind,SchemaTokenKind::NodeStart{name} if name=="namespace") {
            for token in &tokens[index + 1..] {
                match &token.kind {
                    SchemaTokenKind::Attribute { name, value, .. } if name == "uri" => {
                        namespaces.push(value.clone().unwrap());
                        break;
                    }
                    SchemaTokenKind::NodeEnd { .. } => break,
                    _ => {}
                }
            }
        }
        if matches!(&token.kind,SchemaTokenKind::Attribute{name,value:Some(value),..} if name=="name"&&value=="context-root")
        {
            for token in &tokens[index + 1..] {
                match &token.kind {
                    SchemaTokenKind::Attribute { name, value, .. } if name == "children" => {
                        declared = value.clone();
                        break;
                    }
                    SchemaTokenKind::NodeEnd { .. } => break,
                    _ => {}
                }
            }
        }
    }
    assert_eq!(
        declared.unwrap().split_whitespace().collect::<Vec<_>>(),
        PARTS.iter().map(|(name, _)| *name).collect::<Vec<_>>()
    );
    assert_eq!(
        namespaces,
        std::iter::once(NAMESPACE)
            .chain(PARTS.iter().map(|(_, ns)| *ns))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    );
    assert!(
        include_str!("../../../cem-elements/src/lib/cem-elements.ts")
            .contains(&format!("SNAPSHOT_SCHEMA_VERSION = '{VERSION}'"))
    );
}
#[test]
fn native_payload_slots_keep_reference_identity_source_and_order() {
    let bytes = capsule(SOURCE);
    let data = bind(
        &bytes,
        "island-one",
        &super::super::render_control().unwrap(),
    )
    .unwrap();
    let [Item::Record(fields)] = data.bindings["datadom"].items.as_slice() else {
        panic!("missing frame")
    };
    let [Item::Record(slots)] = fields["slots"].as_slice() else {
        panic!("missing slots")
    };
    assert_eq!(slots["heading"].len(), 2);
    let payload = &fields["payload"][0];
    let children = children(payload).unwrap();
    assert_eq!(
        children[0].view().unwrap().identity(),
        slots["heading"][0].view().unwrap().identity()
    );
    assert_eq!(
        children[1].view().unwrap().identity(),
        slots["heading"][1].view().unwrap().identity()
    );
    assert_eq!(
        field(&slots["heading"][0], "name").as_deref(),
        Some("strong")
    );
    assert_eq!(field(&slots["heading"][1], "name").as_deref(), Some("span"));
    assert_eq!(field(&slots[""][0], "value").as_deref(), Some("Tail"));
    assert_eq!(
        slots["heading"][0]
            .view()
            .unwrap()
            .provenance()
            .unwrap()
            .source_uri
            .as_deref(),
        Some("app://island/source.xhtml")
    );
    assert!(slots["heading"][0].source_map().unwrap().frames.len() > 0);
    assert_eq!(
        decode_values(&bytes, &Default::default())
            .unwrap()
            .items
            .len(),
        1
    );
}
#[test]
fn malformed_or_unimplemented_state_refuses_without_echoing_content() {
    let cases=[
        SOURCE.replace("version=\"0.2.0\"","version=\"0.1.2\""),
        SOURCE.replace("runtime/data-island","runtime/foreign"),
        SOURCE.replace("runtime/instance-slices","runtime/foreign"),
        SOURCE.replace(" <a:attributes/>\n <d:dataset/>"," <d:dataset/>\n <a:attributes/>"),
        SOURCE.replace("<s:slices/>",""),
        SOURCE.replace("<s:slices/>","<s:slices/><s:slices/>"),
        SOURCE.replace("<s:slices/>","<s:slices><s:slice name=\"secret\" kind=\"string\">private credential</s:slice></s:slices>"),
        SOURCE.replace("content-type=\"text/html\"","content-type=\"application/json\""),
        SOURCE.replace("https://cem.dev/ns/data/html/1","https://cem.dev/ns/foreign"),
        SOURCE.replace("<f:form-state/>","<f:form-state><f:field name=\"password\" kind=\"string\">private credential</f:field></f:form-state>"),
        SOURCE.replace("Tail</p:payload>","<html:input type=\"password\" value=\"private credential\"/></p:payload>"),
        SOURCE.replace("Tail</p:payload>","<html:input autocomplete=\"one-time-code\"/></p:payload>"),
        SOURCE.replace("xmlns:html=\"http://www.w3.org/1999/xhtml\"","xmlns:html=\"urn:foreign\""),
    ];
    for source in cases {
        assert_eq!(
            bind(
                &capsule(&source),
                "island-one",
                &super::super::render_control().unwrap()
            )
            .unwrap_err(),
            INVALID
        );
    }
    let bytes = capsule(SOURCE);
    assert_eq!(
        bind(
            &bytes,
            "other-instance",
            &super::super::render_control().unwrap()
        )
        .unwrap_err(),
        INVALID
    );
    assert!(bind(
        b"private credential",
        "island-one",
        &super::super::render_control().unwrap()
    )
    .is_err());
    assert!(bind(
        &bytes,
        "island-one",
        &super::super::render_control().unwrap()
    )
    .is_ok());
}

#[test]
fn missing_source_metadata_and_multiple_roots_refuse_but_prefixes_do_not_define_identity() {
    use cem_ml::value::artifact::CemValueGraph;
    let bytes = capsule(SOURCE);
    for mutation in 0..3 {
        let mut graph = CemValueGraph::decode(&bytes, &Default::default()).unwrap();
        let node = graph
            .records
            .iter_mut()
            .find(|node| node.kind == "element" && node.name == "strong")
            .unwrap();
        match mutation {
            0 => node.provenance = None,
            1 => node.provenance.as_mut().unwrap().source_key = None,
            _ => {
                node.provenance.as_mut().unwrap().source_uri =
                    Some("data:private credential".into())
            }
        }
        assert_eq!(
            bind(
                &graph.encode(&Default::default()).unwrap(),
                "island-one",
                &super::super::render_control().unwrap()
            )
            .unwrap_err(),
            INVALID
        );
    }
    let mut graph = CemValueGraph::decode(&bytes, &Default::default()).unwrap();
    graph.roots.push(graph.roots[0]);
    assert!(bind(
        &graph.encode(&Default::default()).unwrap(),
        "island-one",
        &super::super::render_control().unwrap()
    )
    .is_err());
    let renamed = SOURCE
        .replace("i:context-root", "root:context-root")
        .replace("xmlns:i=", "xmlns:root=");
    assert!(bind(
        &capsule(&renamed),
        "island-one",
        &super::super::render_control().unwrap()
    )
    .is_ok());
    let proto = SOURCE.replacen("slot=\"heading\"", "slot=\"__proto__\"", 1);
    let data = bind(
        &capsule(&proto),
        "island-one",
        &super::super::render_control().unwrap(),
    )
    .unwrap();
    let [Item::Record(fields)] = data.bindings["datadom"].items.as_slice() else {
        panic!()
    };
    let [Item::Record(slots)] = fields["slots"].as_slice() else {
        panic!()
    };
    assert_eq!(slots["__proto__"].len(), 1);
    assert_eq!(slots["heading"].len(), 1);
}
