//! Producer-owned collection control projections. These are explicit publication
//! records, never a parser or a replacement for retained graph/AST owners.
use crate::engine::{TransformGraphArtifact, TransformGraphPrimaryKind};
use serde_json::{json, Value};

fn control(artifact: &TransformGraphArtifact) -> Option<&Value> {
    (artifact.primary_kind == TransformGraphPrimaryKind::Collection
        && artifact.primary.get("kind").and_then(Value::as_str) == Some("collection"))
    .then_some(&artifact.primary)
}

fn field(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or(Value::Null)
}

/// Stable manifest value. Provenance remains in the native response and the
/// host's established metadata channel; application values are copied intact.
pub fn manifest(artifact: &TransformGraphArtifact) -> Option<Value> {
    let primary = control(artifact)?;
    let items = primary
        .get("items")?
        .as_array()?
        .iter()
        .map(|item| {
            json!({
                "input": field(item, "input"), "artifactId": field(item, "artifactId"),
                "uri": field(item, "uri"), "destination": field(item, "destination"),
                "identity": field(item, "identity"), "primary": field(item, "primary"),
                "bindings": item.get("bindings").cloned().unwrap_or_else(|| json!({})),
            })
        })
        .collect::<Vec<_>>();
    Some(
        json!({ "kind": "collection", "mode": field(primary, "mode"),
        "count": primary.get("count").cloned().unwrap_or_else(|| json!(items.len())),
        "bindings": primary.get("bindings").cloned().unwrap_or_else(|| json!({})), "items": items }),
    )
}

pub fn collection_source_map(artifact: &TransformGraphArtifact) -> Option<Value> {
    let primary = control(artifact)?;
    let items = primary
        .get("items")?
        .as_array()?
        .iter()
        .filter_map(|item| {
            let source_map = item.get("sourceMap").filter(|map| !map.is_null())?;
            Some(
                json!({ "input": field(item, "input"), "artifactId": field(item, "artifactId"),
            "uri": field(item, "uri"), "destination": field(item, "destination"),
            "identity": field(item, "identity"), "sourceMap": source_map,
            "outputSpans": item.get("outputSpans").cloned().unwrap_or_else(|| json!([])) }),
            )
        })
        .collect::<Vec<_>>();
    (!items.is_empty()).then(|| json!({ "kind": "collection", "items": items }))
}

pub fn has_control_metadata(artifact: &TransformGraphArtifact) -> bool {
    artifact.primary_kind == TransformGraphPrimaryKind::Collection
        || !super::is_declared_json(artifact.identity.as_ref())
}

pub fn source_map(artifact: &TransformGraphArtifact) -> Option<Value> {
    artifact
        .source_map
        .as_ref()
        .and_then(|map| serde_json::to_value(map).ok())
        .or_else(|| {
            has_control_metadata(artifact)
                .then(|| {
                    artifact
                        .primary
                        .get("sourceMap")
                        .filter(|map| !map.is_null())
                        .cloned()
                })
                .flatten()
        })
        .or_else(|| collection_source_map(artifact))
}

pub fn output_spans(artifact: &TransformGraphArtifact) -> Value {
    if artifact.source_map.is_some() || !artifact.output_spans.is_empty() {
        return serde_json::to_value(&artifact.output_spans).unwrap_or_else(|_| json!([]));
    }
    if has_control_metadata(artifact) {
        if let Some(spans) = artifact
            .primary
            .get("outputSpans")
            .filter(|spans| spans.is_array())
        {
            return spans.clone();
        }
    }
    json!([])
}

pub fn sidecar(
    source_map: &Value,
    export_id: &str,
    input: &str,
    destination: &str,
    output_spans: Value,
) -> Value {
    let mut sidecar = source_map.clone();
    if let Value::Object(fields) = &mut sidecar {
        fields.insert("exportId".into(), json!(export_id));
        fields.insert("input".into(), json!(input));
        fields.insert("destination".into(), json!(destination));
        fields.insert("outputSpans".into(), output_spans);
    }
    sidecar
}

pub fn item_sidecar(
    source_map: &Value,
    artifact: &TransformGraphArtifact,
    item: &Value,
    destination: &str,
) -> Option<Value> {
    control(artifact)?;
    let mut sidecar = self::sidecar(
        source_map,
        &artifact.export_id,
        item.get("input")
            .and_then(Value::as_str)
            .unwrap_or(&artifact.input),
        destination,
        item.get("outputSpans")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    if let Value::Object(fields) = &mut sidecar {
        for (key, value) in ["artifactId", "uri"].into_iter().filter_map(|key| {
            item.get(key)
                .and_then(Value::as_str)
                .map(|value| (key, value))
        }) {
            fields.insert(key.into(), json!(value));
        }
        if let Some(destination) = &artifact.destination {
            fields.insert("collectionDestination".into(), json!(destination));
        }
    }
    Some(sidecar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{FormatIdentity, PrimaryBytes};

    fn artifact() -> TransformGraphArtifact {
        TransformGraphArtifact {
            primary_kind: TransformGraphPrimaryKind::Collection,
            export_id: "site".into(),
            input: "pages".into(),
            destination: Some("out/site.json".into()),
            identity: Some(FormatIdentity {
                content_type: Some("application/json".into()),
                ..Default::default()
            }),
            primary: serde_json::from_slice(include_bytes!(
                "../../tests/fixtures/graph-output/collection-control.json"
            ))
            .unwrap(),
            primary_bytes: None,
            source_map: None,
            output_spans: vec![],
        }
    }

    #[test]
    fn native_collection_manifest_matches_golden_and_does_not_mutate_inline_metadata() {
        let artifact = artifact();
        let original = artifact.primary.clone();
        let expected = include_bytes!("../../tests/fixtures/graph-output/collection-manifest.json");
        assert_eq!(
            super::super::graph_artifact_bytes(&artifact)
                .unwrap()
                .unwrap()
                .as_ref(),
            expected
        );
        assert_eq!(artifact.primary, original);
        let spans: Vec<crate::interpreter::OutputSpan> =
            serde_json::from_value(original["items"][0]["outputSpans"].clone()).unwrap();
        assert_eq!(spans[0].origin.frames[0].source_id.0, 7);
        let mut lookalike = artifact.clone();
        lookalike.primary_kind = TransformGraphPrimaryKind::Value;
        assert!(manifest(&lookalike).is_none());
        assert!(collection_source_map(&lookalike).is_none());
        assert!(item_sidecar(
            &original["items"][0]["sourceMap"],
            &lookalike,
            &original["items"][0],
            "out/index.html"
        )
        .is_none());
        assert_eq!(
            super::super::graph_artifact_bytes(&lookalike)
                .unwrap()
                .unwrap()
                .as_ref(),
            serde_json::to_vec_pretty(&original).unwrap()
        );
    }

    #[test]
    fn sidecars_keep_export_child_and_original_source_owners_and_spans() {
        let artifact = artifact();
        let item = &artifact.primary["items"][0];
        let aggregate = source_map(&artifact).unwrap();
        assert_eq!(aggregate["items"][0]["sourceMap"], item["sourceMap"]);
        assert_eq!(aggregate["items"][0]["outputSpans"], item["outputSpans"]);
        let top = sidecar(
            &aggregate,
            &artifact.export_id,
            &artifact.input,
            "out/site.json",
            json!([]),
        );
        assert_eq!(top["exportId"], "site");
        assert_eq!(top["input"], "pages");
        assert_eq!(top["items"][0]["uri"], "memory:page.cem");
        let child = item_sidecar(&item["sourceMap"], &artifact, item, "out/index.html").unwrap();
        assert_eq!(child["exportId"], "site");
        assert_eq!(child["input"], "page-source");
        assert_eq!(child["artifactId"], "page");
        assert_eq!(child["uri"], "memory:page.cem");
        assert_eq!(child["collectionDestination"], "out/site.json");
        assert_eq!(child["destination"], "out/index.html");
        assert_eq!(child["frames"], item["sourceMap"]["frames"]);
        assert_eq!(child["outputSpans"], item["outputSpans"]);
    }

    #[test]
    fn empty_controls_have_no_invented_provenance_and_explicit_writers_win() {
        let mut artifact = artifact();
        artifact.primary = json!({"kind":"collection", "items":[]});
        assert_eq!(
            manifest(&artifact).unwrap(),
            json!({"kind":"collection","mode":null,"count":0,"bindings":{},"items":[]})
        );
        assert!(source_map(&artifact).is_none());
        assert_eq!(output_spans(&artifact), json!([]));
        artifact.primary_bytes = Some(PrimaryBytes {
            content_type: "application/octet-stream".into(),
            schema: None,
            format_version: "native/1".into(),
            hash_scheme: "sha256".into(),
            hash: crate::command_service::sha256_hex(&[]),
            bytes: vec![],
        });
        assert!(
            matches!(super::super::graph_artifact_bytes(&artifact).unwrap(), Some(std::borrow::Cow::Borrowed(bytes)) if bytes.is_empty())
        );
    }
}
