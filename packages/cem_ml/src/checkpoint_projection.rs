//! Explicit renderer checkpoint transport with opaque native value bytes, never
//! an evaluation AST, authored input, live DOM serialization or native authority.
use crate::checkpoint_values::{sensitive, NativeArtifact};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    version: String,
    origin: Origin,
    entries: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "nativeArtifacts")]
    native_artifacts: Vec<NativeArtifact>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Origin {
    instance_id: String,
    template_artifact_id: String,
    data_revision: String,
    scope_policy_stamp: String,
}
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Entry {
    Element {
        node_id: String,
        tag: String,
        namespace: Option<String>,
        attributes: Vec<Attribute>,
    },
    Text {
        node_id: String,
        text: String,
    },
    Comment {
        node_id: String,
        text: String,
    },
    End {},
    RangeStart {
        node_id: String,
        range_id: String,
    },
    RangeEnd {
        node_id: String,
        range_id: String,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attribute {
    name: String,
    value: String,
    #[serde(rename = "nativeValue", skip_serializing_if = "Option::is_none")]
    native_value: Option<NativeValue>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeValue {
    artifact: usize,
    index: usize,
}
fn identity(s: &str) -> bool {
    !s.is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
}
fn name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 1024
        && s.chars().enumerate().all(|(i, c)| {
            c.is_alphabetic()
                || c == '_'
                || i > 0 && (c.is_ascii_digit() || matches!(c, '-' | '.' | ':'))
        })
}
impl Projection {
    pub fn validate(
        &self,
        roots: &[String],
        bytes: &mut usize,
    ) -> Result<BTreeSet<&str>, &'static str> {
        let fail = "cem.checkpoint.runtime.projection";
        for field in [
            &self.origin.instance_id,
            &self.origin.template_artifact_id,
            &self.origin.data_revision,
            &self.origin.scope_policy_stamp,
        ] {
            if !identity(field) {
                return Err(fail);
            }
            *bytes = bytes.saturating_add(field.len());
        }
        if self.version != "cem-render-projection/1" || self.entries.len() > 4096 {
            return Err(fail);
        }
        if self.native_artifacts.len() > 64 {
            return Err(fail);
        }
        let graphs = self
            .native_artifacts
            .iter()
            .map(|artifact| artifact.decode(bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let mut used_artifacts = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        if self
            .native_artifacts
            .iter()
            .any(|a| !hashes.insert(&a.content_hash))
        {
            return Err(fail);
        }
        let mut ids = BTreeSet::new();
        let mut elements = BTreeSet::new();
        let mut found = Vec::new();
        let mut depth = 0usize;
        let mut range: Option<(&str, usize, bool)> = None;
        for entry in &self.entries {
            let id = match entry {
                Entry::Element { node_id, .. }
                | Entry::Text { node_id, .. }
                | Entry::Comment { node_id, .. }
                | Entry::RangeStart { node_id, .. }
                | Entry::RangeEnd { node_id, .. } => Some(node_id),
                Entry::End {} => None,
            };
            if let Some(id) = id {
                if !identity(id) || !ids.insert(id.as_str()) {
                    return Err(fail);
                }
                if depth == 0 {
                    found.push(id.clone());
                }
            }
            match entry {
                Entry::Element {
                    node_id,
                    tag,
                    namespace,
                    attributes,
                } => {
                    if range.is_some()
                        || depth >= 64
                        || !name(tag)
                        || attributes.len() > 128
                        || namespace.as_deref().is_some_and(|s| {
                            !matches!(
                                s,
                                "http://www.w3.org/1999/xhtml"
                                    | "http://www.w3.org/2000/svg"
                                    | "http://www.w3.org/1998/Math/MathML"
                            )
                        })
                    {
                        return Err(fail);
                    }
                    let mut names = BTreeSet::new();
                    for a in attributes {
                        if sensitive(&a.name, &a.value) {
                            return Err(fail);
                        }
                        if let Some(value) = &a.native_value {
                            let graph = graphs.get(value.artifact).ok_or(fail)?;
                            let root = graph.roots.get(value.index).ok_or(fail)?;
                            let record = &graph.records[*root as usize];
                            if record.kind != "attribute"
                                || record.name != a.name
                                || !record.has_native_content()
                            {
                                return Err(fail);
                            }
                            used_artifacts.insert(value.artifact);
                        }
                        if !name(&a.name) || !names.insert(&a.name) {
                            return Err(fail);
                        }
                        *bytes = bytes.saturating_add(a.name.len() + a.value.len());
                    }
                    *bytes = bytes.saturating_add(node_id.len() + tag.len());
                    elements.insert(node_id.as_str());
                    depth += 1;
                }
                Entry::Text { node_id, text } | Entry::Comment { node_id, text } => {
                    if let Some((id, _, seen)) = &mut range {
                        if *seen || *id != node_id {
                            return Err(fail);
                        }
                        *seen = true;
                    }
                    *bytes = bytes.saturating_add(node_id.len() + text.len());
                }
                Entry::End {} => {
                    if depth == 0 || range.is_some() {
                        return Err(fail);
                    }
                    depth -= 1;
                }
                Entry::RangeStart { node_id, range_id } => {
                    if range.is_some()
                        || !identity(range_id)
                        || node_id != &format!("{range_id}:start")
                    {
                        return Err(fail);
                    }
                    range = Some((range_id, depth, false));
                    *bytes = bytes.saturating_add(node_id.len() + range_id.len());
                }
                Entry::RangeEnd { node_id, range_id } => {
                    if range != Some((range_id.as_str(), depth, true))
                        || node_id != &format!("{range_id}:end")
                    {
                        return Err(fail);
                    }
                    range = None;
                    *bytes = bytes.saturating_add(node_id.len() + range_id.len());
                }
            }
            if *bytes > 256 * 1024 {
                return Err(fail);
            }
        }
        if depth != 0 || range.is_some() || found != roots || used_artifacts.len() != graphs.len() {
            return Err(fail);
        }
        Ok(elements)
    }
}
