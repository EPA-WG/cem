//! Immutable outputs and observations of the pinned historical native encoders.
use cem_ml::{
    ast::decode::DebugBinaryDecoder,
    parser::{document::CemDocument, CemAstNode},
    source_map::SourceMapStack,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cemb-legacy")
}

fn checked(path: &str, digest: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(path)).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        digest.as_str().unwrap(),
        "historical fixture checksum changed: {path}"
    );
    bytes
}

pub fn bytes(version: u16, name: &str) -> Vec<u8> {
    let manifest: Value =
        serde_json::from_slice(&fs::read(root().join("provenance.json")).unwrap()).unwrap();
    assert_eq!(manifest["schemaVersion"], 1);
    checked("produce.rs", &manifest["driverSha256"]);
    let path = format!("v{version}-{name}.cemb");
    let entry = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == path)
        .expect("historical fixture provenance missing");
    assert_eq!(entry["wireVersion"], version);
    checked(entry["source"].as_str().unwrap(), &entry["sourceSha256"]);
    checked(
        entry["expected"].as_str().unwrap(),
        &entry["expectedSha256"],
    );
    let bytes = checked(&path, &entry["sha256"]);
    assert_eq!(bytes.len() as u64, entry["bytes"].as_u64().unwrap());
    assert_eq!(&bytes[..4], b"CEMB");
    assert_eq!(u16::from_le_bytes(bytes[4..6].try_into().unwrap()), version);
    bytes
}

pub fn decode(version: u16, name: &str) -> CemDocument {
    let document = DebugBinaryDecoder::new()
        .decode(&bytes(version, name))
        .unwrap();
    let expected: Value = serde_json::from_slice(
        &fs::read(root().join(format!("v{version}-{name}.expected.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        document.nodes.len() as u64,
        expected["nodeCount"].as_u64().unwrap()
    );
    assert_eq!(
        cem_ml::formatter::format(&document),
        expected["canonicalSource"].as_str().unwrap()
    );
    assert!(document.format_identity.is_none());
    assert!(document.typed_preludes.is_empty());
    let attributes = expected["attributes"].as_object().unwrap();
    let references = expected["references"].as_object().unwrap();
    assert_eq!(
        document
            .nodes
            .iter()
            .filter(|node| matches!(node, CemAstNode::Attribute { .. }))
            .count(),
        attributes.len()
    );
    assert_eq!(
        document
            .nodes
            .iter()
            .filter(|node| matches!(node, CemAstNode::Reference { .. }))
            .count(),
        references.len()
    );
    for (id, expected) in attributes {
        let CemAstNode::Attribute {
            expanded_name,
            value,
            value_nodes,
            source,
            ..
        } = document.get(id.parse().unwrap()).unwrap()
        else {
            panic!("historical attribute identity lost")
        };
        assert_eq!(expanded_name.local_name, expected["name"].as_str().unwrap());
        assert_eq!(
            value,
            &serde_json::from_value::<Option<String>>(expected["value"].clone()).unwrap()
        );
        assert!(
            value_nodes.is_empty(),
            "legacy literal acquired native value nodes"
        );
        assert_eq!(
            source,
            &serde_json::from_value::<SourceMapStack>(expected["source"].clone()).unwrap()
        );
    }
    for (id, expected) in references {
        let CemAstNode::Reference {
            expression,
            context,
            targets,
            source,
            ..
        } = document.get(id.parse().unwrap()).unwrap()
        else {
            panic!("historical reference identity lost")
        };
        assert_eq!(expression, expected["expression"].as_str().unwrap());
        assert_eq!(*context as u64, expected["context"].as_u64().unwrap());
        assert_eq!(
            targets,
            &serde_json::from_value::<Option<Vec<u32>>>(expected["targets"].clone()).unwrap()
        );
        assert_eq!(
            source,
            &serde_json::from_value::<SourceMapStack>(expected["source"].clone()).unwrap()
        );
    }
    document
}
