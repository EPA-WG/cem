//! Compiled only inside the pinned historical checkouts by regenerate.py.
use cem_ml::{
    ast::{decode::DebugBinaryDecoder, encode::DebugBinaryEncoder, format::VERSION},
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

fn parse(source: &str) -> CemDocument {
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
    )))
    .build();
    assert!(
        document.diagnostics.is_empty(),
        "{:?}",
        document.diagnostics
    );
    document
}

fn emit(directory: &Path, name: &str, document: CemDocument) {
    let mut attributes = BTreeMap::new();
    #[allow(unused_mut)]
    let mut references: BTreeMap<u32, Value> = BTreeMap::new();
    for node in &document.nodes {
        if let CemAstNode::Attribute {
            node_id,
            expanded_name,
            value,
            source,
            ..
        } = node
        {
            attributes.insert(
                *node_id,
                json!({ "name": expanded_name.local_name,
                "value": value, "source": source }),
            );
        }
        #[cfg(legacy_references)]
        if let CemAstNode::Reference {
            node_id,
            expression,
            context,
            targets,
            source,
        } = node
        {
            references.insert(
                *node_id,
                json!({ "expression": expression,
                "context": context, "targets": targets, "source": source }),
            );
        }
    }
    let bytes = DebugBinaryEncoder::new().encode(&document).bytes;
    assert_eq!(u16::from_le_bytes(bytes[4..6].try_into().unwrap()), VERSION);
    // The unchanged historical decoder independently accepts its own output.
    let decoded = DebugBinaryDecoder::new().decode(&bytes).unwrap();
    assert_eq!(decoded.nodes.len(), document.nodes.len());
    let stem = format!("v{VERSION}-{name}");
    fs::write(directory.join(format!("{stem}.cemb")), bytes).unwrap();
    let expected = json!({ "wireVersion": VERSION, "nodeCount": document.nodes.len(),
        "canonicalSource": cem_ml::formatter::format(&document),
        "attributes": attributes, "references": references });
    fs::write(
        directory.join(format!("{stem}.expected.json")),
        format!("{}\n", serde_json::to_string_pretty(&expected).unwrap()),
    )
    .unwrap();
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let directory = Path::new(&args[1]);
    assert_eq!(VERSION, args[2].parse::<u16>().unwrap());
    let source = |name: &str| fs::read_to_string(directory.join(format!("{name}.cem"))).unwrap();
    emit(directory, "literal", parse(&source("literal")));
    if VERSION == 2 {
        emit(directory, "ordinary", parse(&source("ordinary")));
    }
    #[cfg(legacy_references)]
    {
        emit(directory, "reference", parse(&source("reference")));
        let mut document = parse(&source("reference-graph"));
        let references: Vec<_> = document
            .nodes
            .iter()
            .filter_map(|node| match node {
                CemAstNode::Reference { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .collect();
        assert_eq!(references.len(), 4);
        let target = document
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "target" => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let edges = [
            Some(vec![target, references[1], target, references[0]]),
            Some(vec![references[0]]),
            Some(vec![]),
            None,
        ];
        for (id, edges) in references.iter().zip(edges) {
            let CemAstNode::Reference { targets, .. } = &mut document.nodes[*id as usize] else {
                unreachable!()
            };
            *targets = edges;
        }
        emit(directory, "reference-graph", document);
    }
}
