//! Opaque native value checkpoints. Decode only at the native import boundary.
use crate::value::artifact::{CemValueArtifactLimits, CemValueGraph};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeArtifact {
    pub(crate) content_hash: String,
    pub(crate) base64: String,
}
pub(crate) fn sensitive(name: &str, value: &str) -> bool {
    name.eq_ignore_ascii_case("type")
        && ["password", "file"]
            .iter()
            .any(|s| value.eq_ignore_ascii_case(s))
        || name.eq_ignore_ascii_case("autocomplete")
            && value.split_whitespace().any(|v| {
                ["current-password", "new-password", "one-time-code"]
                    .iter()
                    .any(|s| v.eq_ignore_ascii_case(s))
            })
}
impl NativeArtifact {
    pub(crate) fn decode(&self, bytes: &mut usize) -> Result<CemValueGraph, &'static str> {
        self.decode_with_limits(
            bytes,
            &CemValueArtifactLimits {
                max_bytes: 256 * 1024,
                max_values: 4096,
                max_depth: 64,
            },
        )
    }
    fn decode_with_limits(
        &self,
        bytes: &mut usize,
        limits: &CemValueArtifactLimits,
    ) -> Result<CemValueGraph, &'static str> {
        let fail = "cem.checkpoint.runtime.projection.native";
        let decoded = STANDARD.decode(&self.base64).map_err(|_| fail)?;
        *bytes = bytes.saturating_add(decoded.len());
        if *bytes > 256 * 1024
            || crate::content_cache::ContentHash::from_blake3(&decoded).header_value()
                != self.content_hash
        {
            return Err(fail);
        }
        let graph = CemValueGraph::decode(
            &decoded,
            &CemValueArtifactLimits {
                max_bytes: limits.max_bytes.min(256 * 1024),
                max_values: limits.max_values.min(4096),
                max_depth: limits.max_depth.min(64),
            },
        )
        .map_err(|_| fail)?;
        if graph.accounted_bytes() > 256 * 1024 {
            return Err(fail);
        }
        for (id, record) in graph.records.iter().enumerate() {
            if record.kind == "attribute"
                && (record.name.eq_ignore_ascii_case("type")
                    || record.name.eq_ignore_ascii_case("autocomplete"))
                && sensitive(
                    &record.name,
                    &graph
                        .string_value(id as u32, 256 * 1024)
                        .map_err(|_| fail)?,
                )
            {
                return Err(fail);
            }
            if record
                .provenance
                .as_ref()
                .and_then(|p| p.source_uri.as_ref())
                .is_some_and(|uri| {
                    url::Url::parse(uri)
                        .is_ok_and(|url| !url.username().is_empty() || url.password().is_some())
                })
            {
                return Err(fail);
            }
        }
        Ok(graph)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Capsule {
    artifact: NativeArtifact,
    index: usize,
}
impl Capsule {
    pub(crate) fn validate(&self, bytes: &mut usize) -> Result<(), &'static str> {
        let graph = self.artifact.decode(bytes)?;
        graph
            .roots
            .get(self.index)
            .ok_or("cem.checkpoint.runtime.native-input")?;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bindings {
    version: String,
    artifacts: Vec<NativeArtifact>,
    bindings: Vec<Binding>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    target: String,
    name: String,
    artifact: usize,
    index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribute: Option<String>,
}
impl Bindings {
    fn validate(
        &self,
        bytes: &mut usize,
        limits: &CemValueArtifactLimits,
    ) -> Result<(), &'static str> {
        let fail = "cem.checkpoint.runtime.native-input";
        if self.version != "cem-native-bindings/1"
            || self.artifacts.len() > 64
            || self.bindings.len() > 256
        {
            return Err(fail);
        }
        let start_bytes = *bytes;
        let graphs = self
            .artifacts
            .iter()
            .map(|a| a.decode_with_limits(bytes, limits))
            .collect::<Result<Vec<_>, _>>()?;
        if bytes.saturating_sub(start_bytes) > limits.max_bytes {
            return Err(fail);
        }
        let mut hashes = BTreeSet::new();
        if self
            .artifacts
            .iter()
            .any(|a| !hashes.insert(&a.content_hash))
        {
            return Err(fail);
        }
        let mut used = BTreeSet::new();
        let mut names = BTreeSet::new();
        for b in &self.bindings {
            if b.name.is_empty()
                || b.name.len() > 1024
                || b.name.chars().any(char::is_control)
                || !names.insert((&b.target, &b.name))
            {
                return Err(fail);
            }
            let graph = graphs.get(b.artifact).ok_or(fail)?;
            let id = *graph.roots.get(b.index).ok_or(fail)?;
            let record = graph.records.get(id as usize).ok_or(fail)?;
            let wrapper = match b.target.as_str() {
                "attribute" if b.attribute.is_none() => Some(&b.name),
                "slice" => b.attribute.as_ref(),
                _ => return Err(fail),
            };
            if let Some(name) = wrapper {
                if name.is_empty()
                    || name.len() > 1024
                    || record.kind != "attribute"
                    || &record.name != name
                {
                    return Err(fail);
                }
            }
            used.insert(b.artifact);
            *bytes =
                bytes.saturating_add(b.name.len() + b.attribute.as_ref().map_or(0, String::len));
        }
        if used.len() != graphs.len() || *bytes > 256 * 1024 {
            return Err(fail);
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Preparation {
    data_json: String,
    limits: Limits,
    native_bindings: Bindings,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Limits {
    max_bytes: usize,
    max_values: usize,
    max_depth: usize,
}
impl Preparation {
    pub(crate) fn validate(&self, bytes: &mut usize) -> Result<(), &'static str> {
        let fail = "cem.checkpoint.runtime.preparation";
        *bytes = bytes.saturating_add(self.data_json.len());
        if *bytes > 256 * 1024
            || self.data_json.len() > self.limits.max_bytes
            || self.limits.max_bytes == 0
            || self.limits.max_bytes > 16 * 1024 * 1024
            || self.limits.max_values == 0
            || self.limits.max_values > 100_000
            || self.limits.max_depth == 0
            || self.limits.max_depth > 128
        {
            return Err(fail);
        }
        // Existing named native-session control-frame JSON boundary. Original
        // bytes re-enter the common importer; never project/import a JS AST.
        serde_json::from_str::<std::collections::BTreeMap<String, serde::de::IgnoredAny>>(
            &self.data_json,
        )
        .map_err(|_| fail)?;
        crate::import::documents::CemDocuments::default()
            .retain(
                self.data_json.as_bytes(),
                "application/json",
                "memory:checkpoint-preparation.json",
            )
            .map_err(|_| fail)?;
        self.native_bindings.validate(
            bytes,
            &CemValueArtifactLimits {
                max_bytes: self.limits.max_bytes,
                max_values: self.limits.max_values,
                max_depth: self.limits.max_depth,
            },
        )
    }
}
