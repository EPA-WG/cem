//! Shared graph publication bytes. Explicit native writers are authoritative.
//!
//! A declared JSON export without writer bytes uses two-space indentation, LF,
//! UTF-8, native serde_json value/map ordering and escaping, and no BOM or final
//! newline. Its complete abstract value is JSON data, including string values
//! and fields named `content`, `kind` or `items`. This is an explicit output
//! boundary, not a document import or runtime AST representation.
use std::borrow::Cow;

use crate::engine::TransformGraphArtifact;
use crate::schema::registry::content_type_essence;

pub mod collection;
mod structured;
pub use structured::{structured_content_type, structured_document_bytes};

/// Return explicit native bytes, a producer-owned collection manifest, or the
/// shared declared-JSON fallback. Collections use the established format-owned
/// structured writers; ordinary data never acquires a collection role by shape.
pub fn graph_artifact_bytes(
    artifact: &TransformGraphArtifact,
) -> Result<Option<Cow<'_, [u8]>>, serde_json::Error> {
    if let Some(primary) = artifact.primary_bytes.as_ref() {
        return Ok(Some(Cow::Borrowed(&primary.bytes)));
    }
    if let Some(manifest) = collection::manifest(artifact) {
        return structured_document_bytes(&manifest, artifact.identity.as_ref())
            .map(|bytes| Some(Cow::Owned(bytes)))
            .map_err(serde_json::Error::io);
    }
    if artifact.primary_kind == crate::engine::TransformGraphPrimaryKind::Collection
        || !is_declared_json(artifact.identity.as_ref())
    {
        return Ok(None);
    }
    serde_json::to_vec_pretty(&artifact.primary).map(|bytes| Some(Cow::Owned(bytes)))
}

/// Explicit graph JSON identity; parameters and MIME case do not change policy.
pub fn is_declared_json(identity: Option<&crate::engine::FormatIdentity>) -> bool {
    let Some(content_type) = identity.and_then(|identity| identity.content_type.as_deref()) else {
        return false;
    };
    let mime = content_type_essence(content_type);
    matches!(mime.as_str(), "application/json" | "text/json") || mime.ends_with("+json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{FormatIdentity, PrimaryBytes};
    use serde_json::{json, Value};

    fn artifact(value: Value, content_type: Option<&str>) -> TransformGraphArtifact {
        TransformGraphArtifact {
            primary_kind: Default::default(),
            export_id: "json".into(),
            input: "data".into(),
            destination: None,
            identity: content_type.map(|mime| FormatIdentity {
                content_type: Some(mime.into()),
                ..Default::default()
            }),
            primary: value,
            primary_bytes: None,
            source_map: None,
            output_spans: vec![],
        }
    }

    #[test]
    fn declared_json_has_exact_scalar_nested_and_unicode_bytes_without_shape_inference() {
        let nested = include_bytes!("../tests/fixtures/graph-output/abstract-json.json");
        for mime in [
            "application/json",
            " APPLICATION/JSON ; charset=utf-8",
            "application/vnd.example+json;version=1",
            "text/json",
        ] {
            let value: Value = serde_json::from_slice(nested).unwrap();
            let owned = artifact(value, Some(mime));
            assert_eq!(
                graph_artifact_bytes(&owned).unwrap().unwrap().as_ref(),
                nested
            );
        }
        for (value, expected) in [
            (Value::Null, "null"),
            (json!(true), "true"),
            (json!(false), "false"),
            (json!(-42), "-42"),
            (json!(1.25), "1.25"),
            (json!(9007199254740993u64), "9007199254740993"),
            (json!("Café 🍒"), "\"Café 🍒\""),
            (json!([]), "[]"),
            (json!({}), "{}"),
            (json!({"content":"body"}), "{\n  \"content\": \"body\"\n}"),
        ] {
            let owned = artifact(value, Some("application/json"));
            assert_eq!(
                graph_artifact_bytes(&owned).unwrap().unwrap().as_ref(),
                expected.as_bytes()
            );
        }
    }

    #[test]
    fn explicit_writer_bytes_even_empty_are_borrowed_unchanged() {
        for bytes in [
            vec![],
            b"{ \"z\": 1, \"a\": 2 }\r\n".to_vec(),
            vec![0, 255, 10],
        ] {
            let mut owned = artifact(
                json!({"content":"ignore metadata"}),
                Some("application/json"),
            );
            owned.primary_bytes = Some(PrimaryBytes {
                content_type: "application/json".into(),
                schema: None,
                format_version: "native-writer/1".into(),
                hash_scheme: "sha256".into(),
                hash: crate::command_service::sha256_hex(&bytes),
                bytes: bytes.clone(),
            });
            assert!(matches!(
                graph_artifact_bytes(&owned).unwrap(),
                Some(Cow::Borrowed(_))
            ));
            assert_eq!(
                graph_artifact_bytes(&owned).unwrap().unwrap().as_ref(),
                bytes
            );
            assert_eq!(
                owned.primary_bytes.as_ref().unwrap().hash,
                crate::command_service::sha256_hex(&bytes)
            );
        }
    }

    #[test]
    fn native_collection_control_is_distinct_from_lookalike_json_values() {
        let expected = include_bytes!("../tests/fixtures/graph-output/abstract-json.json");
        let mut owned = artifact(
            serde_json::from_slice(expected).unwrap(),
            Some("application/json"),
        );
        assert_eq!(
            graph_artifact_bytes(&owned).unwrap().unwrap().as_ref(),
            expected
        );
        owned.primary_kind = crate::engine::TransformGraphPrimaryKind::Collection;
        let manifest = collection::manifest(&owned).unwrap();
        assert_eq!(
            graph_artifact_bytes(&owned).unwrap().unwrap().as_ref(),
            serde_json::to_vec_pretty(&manifest).unwrap()
        );
        assert!(manifest.get("sourceMap").is_none());
        assert!(manifest["items"][0].get("sourceMap").is_none());
        let encoded = serde_json::to_value(&owned).unwrap();
        assert!(encoded.get("primaryKind").is_none());
        let decoded: TransformGraphArtifact = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            decoded.primary_kind,
            crate::engine::TransformGraphPrimaryKind::Value
        );
    }

    #[test]
    fn non_json_and_unnamed_outputs_do_not_acquire_a_json_encoding() {
        for mime in [
            None,
            Some("text/html"),
            Some("application/yaml"),
            Some("application/notjson"),
            Some("text/plain;name=application/json"),
        ] {
            assert!(
                graph_artifact_bytes(&artifact(json!({"content":"text"}), mime))
                    .unwrap()
                    .is_none()
            );
        }
    }
}
