//! `TemplateRef` and supporting identity types (AC-R-1).
//!
//! Mirrors the design-doc shape in `cem-ml-stack-design-impl.md`
//! §"Template Registry & DCE Integration". The DCE tag-name variant
//! is treated as a registry-owned template reference rather than a
//! browser `customElements` entry — CEM does not police the browser
//! registry in Tier A.

use serde::{Deserialize, Serialize};

/// Opaque schema identity (typically allocated by the schema machine).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaId(pub u32);

/// Opaque registry identity (one per `TemplateRegistry`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegistryId(pub u32);

/// Re-export of the shared source identity so consumers can spell
/// `TemplateRef::LocalId { source_id, .. }` without crossing module
/// boundaries.
pub type SourceId = crate::source::SourceId;

/// Template reference resolved by the registry tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TemplateRef {
    SchemaTemplate {
        schema_id: SchemaId,
        name: String,
    },
    LocalId {
        source_id: SourceId,
        id: String,
    },
    Url {
        url: String,
    },
    UrlFragment {
        url: String,
        fragment: String,
    },
    RegistryEntry {
        registry_id: RegistryId,
        name: String,
    },
    DceTagName {
        tag_name: String,
    },
}

impl TemplateRef {
    /// Stable categorical descriptor for diagnostic / trace output.
    pub fn kind(&self) -> &'static str {
        match self {
            TemplateRef::SchemaTemplate { .. } => "schema-template",
            TemplateRef::LocalId { .. } => "local-id",
            TemplateRef::Url { .. } => "url",
            TemplateRef::UrlFragment { .. } => "url-fragment",
            TemplateRef::RegistryEntry { .. } => "registry-entry",
            TemplateRef::DceTagName { .. } => "dce-tag-name",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    #[test]
    fn kind_strings_are_stable_for_each_variant() {
        let cases = [
            (
                TemplateRef::SchemaTemplate {
                    schema_id: SchemaId(1),
                    name: "Button".into(),
                },
                "schema-template",
            ),
            (
                TemplateRef::LocalId {
                    source_id: SourceId(1),
                    id: "x".into(),
                },
                "local-id",
            ),
            (
                TemplateRef::Url {
                    url: "https://example".into(),
                },
                "url",
            ),
            (
                TemplateRef::UrlFragment {
                    url: "https://example".into(),
                    fragment: "f".into(),
                },
                "url-fragment",
            ),
            (
                TemplateRef::RegistryEntry {
                    registry_id: RegistryId(7),
                    name: "X".into(),
                },
                "registry-entry",
            ),
            (
                TemplateRef::DceTagName {
                    tag_name: "x-card".into(),
                },
                "dce-tag-name",
            ),
        ];
        for (r, expected) in cases {
            assert_eq!(r.kind(), expected);
        }
    }

    #[test]
    fn template_ref_round_trips_through_serde_json() {
        let r = TemplateRef::DceTagName {
            tag_name: "x-card".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        let round: TemplateRef = serde_json::from_value(v).unwrap();
        assert_eq!(round, r);
    }
}
