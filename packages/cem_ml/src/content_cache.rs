//! Shared content-addressed cache and transport primitives
//! (AC-CC-1..AC-CC-7).

use crate::source_map::SourceMapStack;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const HASH_SCHEME: &str = "cem-bin/1+blake3";
pub const FORMAT_VERSION: &str = "cem-cache-artifact/1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentHash {
    pub scheme: String,
    pub hex: String,
}

impl ContentHash {
    pub fn from_blake3(bytes: &[u8]) -> Self {
        Self {
            scheme: HASH_SCHEME.to_owned(),
            hex: blake3::hash(bytes).to_hex().to_string(),
        }
    }

    pub fn header_value(&self) -> String {
        format!("{}:{}", self.scheme, self.hex)
    }

    pub fn to_sidecar_string(&self) -> String {
        format!("{}\n", self.header_value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheMode {
    Dev,
    Prod,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactContentType {
    CemMlDocument,
    CemMlSchema,
    TransformPlan,
    CemQlModule,
    SourceMapSidecar,
    Other(String),
}

impl ArtifactContentType {
    pub fn as_str(&self) -> &str {
        match self {
            ArtifactContentType::CemMlDocument => "cem-ml/document",
            ArtifactContentType::CemMlSchema => "cem-ml/schema",
            ArtifactContentType::TransformPlan => "cem-ml/transform-plan",
            ArtifactContentType::CemQlModule => "cem-ql/module",
            ArtifactContentType::SourceMapSidecar => "cem/source-map",
            ArtifactContentType::Other(value) => value.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyStamps {
    pub declared_schema_uris: BTreeSet<String>,
    pub plugin_imports: BTreeSet<String>,
    pub external_reads: BTreeSet<String>,
    pub scope_policy_fingerprint: String,
}

impl PolicyStamps {
    pub fn new(scope_policy_fingerprint: impl Into<String>) -> Self {
        Self {
            declared_schema_uris: BTreeSet::new(),
            plugin_imports: BTreeSet::new(),
            external_reads: BTreeSet::new(),
            scope_policy_fingerprint: scope_policy_fingerprint.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub content_type: ArtifactContentType,
    pub hash: ContentHash,
    pub mode: CacheMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMapSidecar {
    pub hash: ContentHash,
    pub source_map: SourceMapStack,
}

impl SourceMapSidecar {
    pub fn new(source_map: SourceMapStack) -> Self {
        let bytes = serde_json::to_vec(&source_map).expect("SourceMapStack serializes");
        Self {
            hash: ContentHash::from_blake3(&bytes),
            source_map,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheArtifact {
    pub format_version: String,
    pub key: CacheKey,
    pub policy_stamps: PolicyStamps,
    pub bytes: Vec<u8>,
    /// Dev-mode artifacts retain the sidecar hash; prod artifacts omit
    /// source maps by construction (AC-CC-4 / AC-CC-5).
    pub source_map_sidecar_hash: Option<ContentHash>,
}

impl CacheArtifact {
    pub fn new(
        content_type: ArtifactContentType,
        mode: CacheMode,
        bytes: Vec<u8>,
        policy_stamps: PolicyStamps,
        source_map: Option<SourceMapStack>,
    ) -> (Self, Option<SourceMapSidecar>) {
        let sidecar = match mode {
            CacheMode::Dev => source_map.map(SourceMapSidecar::new),
            CacheMode::Prod => None,
        };
        let key = CacheKey {
            content_type,
            hash: ContentHash::from_blake3(&bytes),
            mode,
        };
        let artifact = Self {
            format_version: FORMAT_VERSION.to_owned(),
            key,
            policy_stamps,
            bytes,
            source_map_sidecar_hash: sidecar.as_ref().map(|s| s.hash.clone()),
        };
        (artifact, sidecar)
    }

    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("CacheArtifact serializes")
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, CacheError> {
        let artifact: Self = serde_json::from_slice(bytes).map_err(|err| CacheError {
            code: "cem.cc.format_version",
            message: err.to_string(),
        })?;
        if artifact.format_version != FORMAT_VERSION {
            return Err(CacheError {
                code: "cem.cc.format_version",
                message: format!(
                    "unsupported cache artifact format {}",
                    artifact.format_version
                ),
            });
        }
        let actual = ContentHash::from_blake3(&artifact.bytes);
        if actual != artifact.key.hash {
            return Err(CacheError {
                code: "cem.cc.hash_mismatch",
                message: "artifact bytes do not match cache key hash".to_owned(),
            });
        }
        Ok(artifact)
    }

    pub fn validate_policy(&self, active: &PolicyStamps) -> Result<(), CacheError> {
        if &self.policy_stamps == active {
            Ok(())
        } else {
            Err(CacheError {
                code: "cem.cc.policy_mismatch",
                message: "cached artifact policy stamps do not match active scope policy".into(),
            })
        }
    }
}

#[derive(Debug, Default)]
pub struct ContentCache {
    artifacts: BTreeMap<CacheKey, CacheArtifact>,
    sidecars: BTreeMap<ContentHash, SourceMapSidecar>,
}

impl ContentCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, artifact: CacheArtifact, sidecar: Option<SourceMapSidecar>) {
        if let Some(sidecar) = sidecar {
            self.sidecars.insert(sidecar.hash.clone(), sidecar);
        }
        self.artifacts.insert(artifact.key.clone(), artifact);
    }

    pub fn get(
        &self,
        key: &CacheKey,
        active_policy: &PolicyStamps,
    ) -> Result<&CacheArtifact, CacheError> {
        let artifact = self.artifacts.get(key).ok_or(CacheError {
            code: "cem.cc.cache_evicted",
            message: "cache artifact is not present".into(),
        })?;
        artifact.validate_policy(active_policy)?;
        Ok(artifact)
    }

    pub fn sidecar(&self, hash: &ContentHash) -> Option<&SourceMapSidecar> {
        self.sidecars.get(hash)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemHashRequest {
    pub uri: String,
    pub if_cem_hash: Option<ContentHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CemHashResponse {
    NotModified {
        cem_hash: ContentHash,
        content_type: ArtifactContentType,
    },
    Body {
        cem_hash: ContentHash,
        content_type: ArtifactContentType,
        body: Vec<u8>,
    },
}

#[derive(Debug, Default)]
pub struct InMemoryCemHashTransport {
    entries: BTreeMap<String, (ArtifactContentType, Vec<u8>, ContentHash)>,
}

impl InMemoryCemHashTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish(
        &mut self,
        uri: impl Into<String>,
        content_type: ArtifactContentType,
        body: Vec<u8>,
    ) -> ContentHash {
        let hash = ContentHash::from_blake3(&body);
        self.entries
            .insert(uri.into(), (content_type, body, hash.clone()));
        hash
    }

    pub fn publish_with_hash(
        &mut self,
        uri: impl Into<String>,
        content_type: ArtifactContentType,
        body: Vec<u8>,
        hash: ContentHash,
    ) {
        self.entries.insert(uri.into(), (content_type, body, hash));
    }

    pub fn fetch(&self, request: &CemHashRequest) -> Result<CemHashResponse, CacheError> {
        let (content_type, body, hash) = self.entries.get(&request.uri).ok_or(CacheError {
            code: "cem.cc.cache_evicted",
            message: format!("{} is not published", request.uri),
        })?;
        if request.if_cem_hash.as_ref() == Some(hash) {
            return Ok(CemHashResponse::NotModified {
                cem_hash: hash.clone(),
                content_type: content_type.clone(),
            });
        }
        Ok(CemHashResponse::Body {
            cem_hash: hash.clone(),
            content_type: content_type.clone(),
            body: body.clone(),
        })
    }
}
