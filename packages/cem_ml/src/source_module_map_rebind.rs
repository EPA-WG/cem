//! Candidate edits for source module-map control documents; never edit authorization.
use crate::command_service::VirtualResourceV1;
use crate::transform_graph_request::{
    SourceModuleMapDeclaredEdgeVerification, TransformGraphResource,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceModuleMapEdit {
    RebindAsset { from_uri: String, to_uri: String },
    RelocateMap { to_uri: String },
}

#[derive(Debug, Clone)]
pub struct SourceModuleMapEditPlan {
    pub before: SourceModuleMapDeclaredEdgeVerification,
    pub after: SourceModuleMapDeclaredEdgeVerification,
    pub rewritten_bytes: Option<Vec<u8>>,
    pub runtime_analysis_required: Vec<String>,
}

/// Plan owned v3 source-map path changes against an immutable asset snapshot.
/// Re-verifies declared edges before and after. Map JSON may be reformatted only
/// when a path changes; asset bytes, identities and bare specifiers never change.
/// All assets remain subject to runtime/document analysis even on success. This
/// API does not acquire resources, modify the snapshot, or authorize host edits.
/// Hosts must also guard resource identities/revisions, update any explicit matching
/// acquisition base with a moved URI, and plan all incoming references separately.
/// The hashes cover bytes, not resolver policy or identity metadata.
pub fn plan_source_module_map_edit(
    resource: TransformGraphResource,
    snapshot: &BTreeMap<String, VirtualResourceV1>,
    edit: SourceModuleMapEdit,
) -> Result<SourceModuleMapEditPlan, String> {
    use crate::transform_graph_request::{
        resolve_transform_graph_reference, verify_source_module_map_declared_edges,
        SourceModuleMapAssetKind,
    };
    const SCHEMA: &str = "https://cem.dev/ns/data/module-map/3";
    if !literal_uri(&resource.uri) {
        return Err("source map requires a canonical absolute acquired URI".into());
    }
    for content_type in resource.content_type.as_deref().into_iter().chain(
        resource
            .identity
            .as_ref()
            .and_then(|id| id.content_type.as_deref()),
    ) {
        if !content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/json")
        {
            return Err("source map identity requires application/json".into());
        }
    }
    if let Some(identity) = &resource.identity {
        if identity
            .schema
            .as_deref()
            .is_some_and(|schema| schema != SCHEMA)
            || identity
                .base_uri
                .as_deref()
                .is_some_and(|base| base != resource.uri)
        {
            return Err("source map identity conflicts with schema or acquisition base".into());
        }
    }
    // This is the explicit JSON module-map control boundary, not a document AST.
    // Typed admission rejects unknown fields and duplicate owners at every level.
    let _: StrictMap = serde_json::from_slice(&resource.bytes).map_err(|e| e.to_string())?;
    let mut authored: serde_json::Value =
        serde_json::from_slice(&resource.bytes).map_err(|e| e.to_string())?;
    let before = verify_source_module_map_declared_edges(resource.clone(), snapshot)
        .map_err(|e| format!("{e:?}"))?;
    if !before.follow_ups.is_empty() {
        return Err(format!(
            "source mechanisms require analysis before path rewriting: {:?}",
            before.follow_ups
        ));
    }
    if before
        .assets
        .iter()
        .any(|item| !literal_uri(&item.asset.source_uri) || item.asset.source_uri == resource.uri)
    {
        return Err("source map assets require non-self literal acquired URIs".into());
    }
    let mut candidate = resource.clone();
    let mut candidate_snapshot = snapshot.clone();
    let mut expected = before.assets.clone();
    match &edit {
        SourceModuleMapEdit::RebindAsset { from_uri, to_uri } => {
            if !literal_uri(from_uri)
                || !literal_uri(to_uri)
                || from_uri == &resource.uri
                || to_uri == &resource.uri
            {
                return Err("asset endpoints require canonical non-map literal URIs".into());
            }
            if from_uri != to_uri
                && expected
                    .iter()
                    .any(|item| &item.asset.source_uri == from_uri)
            {
                if snapshot.contains_key(to_uri) {
                    return Err("asset destination is already occupied in the snapshot".into());
                }
                let source = candidate_snapshot
                    .remove(from_uri)
                    .ok_or("missing source asset")?;
                // An explicit acquired base must move with the resource identity.
                // Custom bases require separate rebinding provenance.
                let mut source = source;
                if let Some(identity) = &mut source.identity {
                    if let Some(base) = &mut identity.base_uri {
                        if base != from_uri {
                            return Err(
                                "asset identity has a custom base requiring analysis".into()
                            );
                        }
                        *base = to_uri.clone();
                    }
                }
                candidate_snapshot.insert(to_uri.clone(), source);
                for item in &mut expected {
                    if &item.asset.source_uri == from_uri {
                        item.asset.source_uri = to_uri.clone();
                    }
                    for edge in &mut item.asset.module_imports {
                        if &edge.target_uri == from_uri {
                            edge.target_uri = to_uri.clone();
                        }
                    }
                }
            }
        }
        SourceModuleMapEdit::RelocateMap { to_uri } => {
            if !literal_uri(to_uri) || (to_uri != &resource.uri && snapshot.contains_key(to_uri)) {
                return Err("map destination requires an unoccupied canonical literal URI".into());
            }
            candidate.uri = to_uri.clone();
            if let Some(identity) = &mut candidate.identity {
                if identity.base_uri.is_some() {
                    identity.base_uri = Some(to_uri.clone());
                }
            }
        }
    }
    let mut changed = false;
    for item in &mut expected {
        let asset = &mut item.asset;
        if resolve_transform_graph_reference(&candidate.uri, &asset.authored_path)
            == asset.source_uri
        {
            continue;
        }
        let collection = match asset.kind {
            SourceModuleMapAssetKind::Import => "imports",
            SourceModuleMapAssetKind::Resource => "resources",
        };
        let slot = authored
            .get_mut(collection)
            .and_then(|items| items.get_mut(&asset.identity))
            .and_then(|item| item.get_mut("path"))
            .ok_or("owned path slot disappeared")?;
        *slot = serde_json::Value::String(asset.source_uri.clone());
        asset.authored_path = asset.source_uri.clone();
        changed = true;
    }
    if changed {
        candidate.bytes = serde_json::to_vec_pretty(&authored).map_err(|e| e.to_string())?;
    }
    let after = verify_source_module_map_declared_edges(candidate.clone(), &candidate_snapshot)
        .map_err(|e| format!("{e:?}"))?;
    if after.assets != expected || after.follow_ups != before.follow_ups {
        return Err(
            "candidate changed asset bytes, identities or declared edges beyond owned paths".into(),
        );
    }
    let runtime_analysis_required = before
        .assets
        .iter()
        .map(|item| item.asset.identity.clone())
        .collect();
    Ok(SourceModuleMapEditPlan {
        before,
        after,
        runtime_analysis_required,
        rewritten_bytes: changed.then_some(candidate.bytes),
    })
}

fn literal_uri(value: &str) -> bool {
    !value
        .chars()
        .any(|ch| ch.is_whitespace() || ch.is_control() || "*{}[]\\".contains(ch))
        && url::Url::parse(value).is_ok_and(|url| {
            !url.cannot_be_a_base()
                && url.query().is_none()
                && url.fragment().is_none()
                && url.as_str() == value
        })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictMap {
    #[serde(rename = "$schema")]
    _schema: String,
    #[serde(rename = "imports", deserialize_with = "unique_entries")]
    _imports: BTreeMap<String, StrictAsset>,
    #[serde(default, rename = "resources", deserialize_with = "unique_entries")]
    _resources: BTreeMap<String, StrictAsset>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictAsset {
    #[serde(rename = "path")]
    _path: String,
    #[serde(rename = "contentType")]
    _content_type: String,
    #[serde(default, rename = "moduleImports", deserialize_with = "unique_entries")]
    _module_imports: BTreeMap<String, String>,
}

fn unique_entries<'de, D, T>(deserializer: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    struct Entries<T>(std::marker::PhantomData<T>);
    impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for Entries<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("module-map entries with unique identity keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut entries = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, T>()? {
                if entries.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate module-map key {key}"
                    )));
                }
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_map(Entries(std::marker::PhantomData))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_service::sha256_hex;
    const MAP: &str = "studio://project/maps/source.json";
    const A: &str = "studio://project/a.js";
    const B: &str = "studio://project/b.js";
    const NEW: &str = "studio://project/moved/b.js";
    fn map() -> TransformGraphResource {
        TransformGraphResource {
            uri: MAP.into(),
            content_type: Some("application/json".into()),
            identity: None,
            bytes: br#"{
  "$schema":"https://cem.dev/ns/data/module-map/3",
  "imports":{
    "a":{"path":"../a.js","contentType":"text/javascript","moduleImports":{"@pkg/b":"b"}},
    "b":{"path":"../b.js","contentType":"text/javascript","moduleImports":{"@pkg/a":"a"}}
  }
}"#
            .to_vec(),
        }
    }
    fn snapshot() -> BTreeMap<String, VirtualResourceV1> {
        BTreeMap::from([
            (
                A.into(),
                VirtualResourceV1 {
                    bytes: b"import '@pkg/b'; export const a = 1;".to_vec(),
                    identity: None,
                },
            ),
            (
                B.into(),
                VirtualResourceV1 {
                    bytes: b"import '@pkg/a'; export const b = 2;".to_vec(),
                    identity: None,
                },
            ),
        ])
    }
    fn rename() -> SourceModuleMapEdit {
        SourceModuleMapEdit::RebindAsset {
            from_uri: B.into(),
            to_uri: NEW.into(),
        }
    }
    #[test]
    fn source_map_edit_rebinds_paths_preserving_identities_cycles_and_asset_bytes() {
        let source = map();
        let snapshot = snapshot();
        let plan = plan_source_module_map_edit(source.clone(), &snapshot, rename()).unwrap();
        let bytes = plan.rewritten_bytes.unwrap();
        let before: serde_json::Value = serde_json::from_slice(&source.bytes).unwrap();
        let after: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let mut expected = before;
        expected["imports"]["b"]["path"] = serde_json::json!(NEW);
        assert_eq!(after, expected);
        assert_eq!(plan.before.map_sha256, sha256_hex(&source.bytes));
        assert_eq!(plan.after.map_sha256, sha256_hex(&bytes));
        assert_eq!(plan.after.assets[0].asset.module_imports[0].target_uri, NEW);
        assert_eq!(plan.after.assets[1].asset.module_imports[0].target_uri, A);
        for (old, new) in plan.before.assets.iter().zip(&plan.after.assets) {
            assert_eq!(old.sha256, new.sha256);
            assert_eq!(old.byte_length, new.byte_length);
        }
        assert_eq!(snapshot[B].bytes, b"import '@pkg/a'; export const b = 2;");
        assert!(!snapshot.contains_key(NEW));
        assert_eq!(plan.runtime_analysis_required, vec!["a", "b"]);
    }
    #[test]
    fn source_map_edit_relocation_preserves_resolved_bindings() {
        let target = "studio://project/deep/maps/source.json";
        let plan = plan_source_module_map_edit(
            map(),
            &snapshot(),
            SourceModuleMapEdit::RelocateMap {
                to_uri: target.into(),
            },
        )
        .unwrap();
        assert_eq!(plan.after.map_uri, target);
        assert_eq!(
            plan.before.assets,
            plan.after
                .assets
                .iter()
                .cloned()
                .map(|mut item| {
                    item.asset.authored_path = plan
                        .before
                        .assets
                        .iter()
                        .find(|old| old.asset.identity == item.asset.identity)
                        .unwrap()
                        .asset
                        .authored_path
                        .clone();
                    item
                })
                .collect::<Vec<_>>()
        );
        let value: serde_json::Value =
            serde_json::from_slice(&plan.rewritten_bytes.unwrap()).unwrap();
        assert_eq!(value["imports"]["a"]["path"], A);
        assert_eq!(value["imports"]["b"]["path"], B);
    }
    #[test]
    fn source_map_edit_noops_retain_exact_map_bytes_and_evidence() {
        for edit in [
            SourceModuleMapEdit::RebindAsset {
                from_uri: B.into(),
                to_uri: B.into(),
            },
            SourceModuleMapEdit::RelocateMap {
                to_uri: "studio://project/maps/renamed.json".into(),
            },
        ] {
            let plan = plan_source_module_map_edit(map(), &snapshot(), edit).unwrap();
            assert!(plan.rewritten_bytes.is_none());
            assert_eq!(plan.before.map_sha256, plan.after.map_sha256);
            assert_eq!(plan.before.assets, plan.after.assets);
        }
    }
    #[test]
    fn source_map_edit_rejects_ambiguous_or_unowned_control_fields() {
        let source = String::from_utf8(map().bytes).unwrap();
        for source in [
            source.replace(
                "\"path\":\"../b.js\"",
                "\"path\":\"ignored.js\",\"path\":\"../b.js\"",
            ),
            source.replace("\"@pkg/b\":\"b\"", "\"@pkg/b\":\"a\",\"@pkg/b\":\"b\""),
            source.replace("\"imports\":{", "\"scopes\":{},\"imports\":{"),
            source.replace(
                "\"contentType\":\"text/javascript\"",
                "\"future\":true,\"contentType\":\"text/javascript\"",
            ),
            source.replace("module-map/3", "module-map/2"),
            source.replace("\"imports\":{", "\"imports\":{},\"imports\":{"),
            source.replace(
                "\"a\":{",
                "\"a\":{\"path\":\"../a.js\",\"contentType\":\"text/javascript\"},\"a\":{",
            ),
        ] {
            let mut resource = map();
            resource.bytes = source.into_bytes();
            assert!(plan_source_module_map_edit(resource, &snapshot(), rename()).is_err());
        }
    }
    #[test]
    fn source_map_edit_evidence_changes_with_bytes_and_checks_identity_bases() {
        use crate::engine::FormatIdentity;
        let mut resources = snapshot();
        let first = plan_source_module_map_edit(map(), &resources, rename()).unwrap();
        resources
            .get_mut(B)
            .unwrap()
            .bytes
            .extend_from_slice(b" // changed");
        let second = plan_source_module_map_edit(map(), &resources, rename()).unwrap();
        assert_ne!(
            first.before.assets[1].sha256,
            second.before.assets[1].sha256
        );
        assert_eq!(
            second.before.assets[1].sha256,
            second.after.assets[1].sha256
        );
        resources.get_mut(B).unwrap().identity = Some(FormatIdentity {
            base_uri: Some("studio://project/custom/base.js".into()),
            ..Default::default()
        });
        assert!(plan_source_module_map_edit(map(), &resources, rename()).is_err());
        for identity in [
            FormatIdentity {
                content_type: Some("text/plain".into()),
                ..Default::default()
            },
            FormatIdentity {
                schema: Some("https://cem.dev/ns/data/module-map/2".into()),
                ..Default::default()
            },
            FormatIdentity {
                base_uri: Some("studio://project/other.json".into()),
                ..Default::default()
            },
        ] {
            let mut source = map();
            source.identity = Some(identity);
            assert!(plan_source_module_map_edit(source, &snapshot(), rename()).is_err());
        }
    }

    #[test]
    fn source_map_edit_rejects_collisions_missing_assets_and_unproven_loads() {
        let mut collided = snapshot();
        collided.insert(
            NEW.into(),
            VirtualResourceV1 {
                bytes: vec![0],
                identity: None,
            },
        );
        assert!(plan_source_module_map_edit(map(), &collided, rename()).is_err());
        assert!(plan_source_module_map_edit(map(), &BTreeMap::new(), rename()).is_err());
        for tail in [
            "import './other.js';",
            "import('other');",
            "const value = `other`;",
        ] {
            let mut assets = snapshot();
            assets
                .get_mut(A)
                .unwrap()
                .bytes
                .extend_from_slice(tail.as_bytes());
            assert!(plan_source_module_map_edit(map(), &assets, rename()).is_err());
        }
        for target in [
            MAP,
            "relative.json",
            "studio://project/*.json",
            "studio://project/new.json#part",
        ] {
            assert!(plan_source_module_map_edit(
                map(),
                &snapshot(),
                SourceModuleMapEdit::RebindAsset {
                    from_uri: B.into(),
                    to_uri: target.into()
                }
            )
            .is_err());
        }
        assert!(plan_source_module_map_edit(
            map(),
            &snapshot(),
            SourceModuleMapEdit::RelocateMap { to_uri: A.into() }
        )
        .is_err());
    }
}
