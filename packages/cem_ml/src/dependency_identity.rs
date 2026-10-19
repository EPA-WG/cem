//! Conservative dependency identities, not cache lookup or reuse authorization.
//! Snapshots contain native/provider-owned fingerprints and revisions, never ASTs,
//! source buffers, DOM, handles or callbacks. Producers attest a complete closure;
//! equal keys do not authorize skipping resource freshness or accepted events.
use crate::content_cache::{ContentHash, HASH_SCHEME};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_DEPENDENCY_IDENTITY_BYTES: usize = 1024 * 1024;
pub const MAX_DEPENDENCY_SOURCES: usize = 1024;
pub const MAX_DEPENDENCY_IMPORTS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyLayer {
    TemplateCompilation,
    Stylesheet,
    RenderProjection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyClosureState {
    Ready,
    Pending,
    Failed,
    Released,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DependencyVersion {
    Content(ContentHash),
    /// Provider identity and generation prevent equal local counters from aliasing.
    Revision {
        owner: String,
        generation: u64,
        revision: u64,
    },
    Opaque,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyInput {
    Data,
    Attributes,
    Payload,
    Slices,
    Resources,
    Form,
    Validation,
    Events,
    Refresh,
    /// Clock/randomness, native/plugin functions and other evaluation providers.
    /// Unknown freshness must be Opaque, even when ordinary UI inputs are stable.
    Providers,
}
pub const RENDER_DEPENDENCY_INPUTS: &[DependencyInput] = &[
    DependencyInput::Data,
    DependencyInput::Attributes,
    DependencyInput::Payload,
    DependencyInput::Slices,
    DependencyInput::Resources,
    DependencyInput::Form,
    DependencyInput::Validation,
    DependencyInput::Events,
    DependencyInput::Refresh,
    DependencyInput::Providers,
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceDependency {
    pub content_type: String,
    pub schema: Option<String>,
    pub version: DependencyVersion,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ImportDependency {
    pub parent_uri: String,
    pub specifier: String,
    pub resolved_uri: String,
}
/// No Default: producers must supply every context explicitly. `sources` contains
/// the complete transitive closure, including the root and provenance-owning URIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencySnapshot {
    pub layer: DependencyLayer,
    pub engine_version: String,
    pub format_version: String,
    pub closure_state: DependencyClosureState,
    pub root_uri: String,
    pub sources: BTreeMap<String, SourceDependency>,
    pub imports: Vec<ImportDependency>,
    pub options: BTreeMap<String, String>,
    pub policy: DependencyVersion,
    pub resolution: DependencyVersion,
    pub scope: DependencyVersion,
    pub inputs: BTreeMap<DependencyInput, DependencyVersion>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyIdentity {
    pub layer: DependencyLayer,
    pub hash: ContentHash,
}
/// Small reason classes, with no retained dependency payload/history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyIdentityError {
    IncompleteClosure,
    OpaqueDependency,
    InvalidContext,
    InvalidClosure,
    InvalidHash,
    MissingInput,
    Bounds,
}

impl DependencySnapshot {
    pub fn identity(&self) -> Result<DependencyIdentity, DependencyIdentityError> {
        use DependencyIdentityError::*;
        if self.closure_state != DependencyClosureState::Ready {
            return Err(IncompleteClosure);
        }
        if self.sources.len() > MAX_DEPENDENCY_SOURCES
            || self.imports.len() > MAX_DEPENDENCY_IMPORTS
            || self.options.len() > 128
        {
            return Err(Bounds);
        }
        let mut budget = 0usize;
        let mut text = |value: &str| {
            budget = budget.checked_add(value.len()).ok_or(Bounds)?;
            if budget > MAX_DEPENDENCY_IDENTITY_BYTES {
                Err(Bounds)
            } else {
                Ok(())
            }
        };
        for value in [&self.engine_version, &self.format_version, &self.root_uri] {
            text(value)?;
            if value.is_empty() {
                return Err(InvalidContext);
            }
        }
        if !self.sources.contains_key(&self.root_uri) {
            return Err(InvalidClosure);
        }
        let version =
            |value: &DependencyVersion,
             text: &mut dyn FnMut(&str) -> Result<(), DependencyIdentityError>| {
                match value {
                    DependencyVersion::Opaque => Err(OpaqueDependency),
                    DependencyVersion::Content(hash) => {
                        text(&hash.scheme)?;
                        text(&hash.hex)?;
                        if hash.scheme != HASH_SCHEME
                            || hash.hex.len() != 64
                            || !hash
                                .hex
                                .bytes()
                                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                        {
                            Err(InvalidHash)
                        } else {
                            Ok(())
                        }
                    }
                    DependencyVersion::Revision { owner, .. } => {
                        text(owner)?;
                        if owner.is_empty() {
                            Err(InvalidContext)
                        } else {
                            Ok(())
                        }
                    }
                }
            };
        for (uri, source) in &self.sources {
            text(uri)?;
            text(&source.content_type)?;
            if uri.is_empty() || source.content_type.is_empty() {
                return Err(InvalidClosure);
            }
            if let Some(schema) = &source.schema {
                text(schema)?;
                if schema.is_empty() {
                    return Err(InvalidContext);
                }
            }
            version(&source.version, &mut text)?;
        }
        let mut aliases = BTreeSet::new();
        for edge in &self.imports {
            for value in [&edge.parent_uri, &edge.specifier, &edge.resolved_uri] {
                text(value)?;
            }
            if edge.specifier.is_empty()
                || !self.sources.contains_key(&edge.parent_uri)
                || !self.sources.contains_key(&edge.resolved_uri)
                || !aliases.insert((&edge.parent_uri, &edge.specifier))
            {
                return Err(InvalidClosure);
            }
        }
        for (name, value) in &self.options {
            text(name)?;
            text(value)?;
            if name.is_empty() {
                return Err(InvalidContext);
            }
        }
        for context in [&self.policy, &self.resolution, &self.scope] {
            version(context, &mut text)?;
        }
        for input in self.inputs.values() {
            version(input, &mut text)?;
        }
        if self.layer == DependencyLayer::RenderProjection
            && RENDER_DEPENDENCY_INPUTS
                .iter()
                .any(|kind| !self.inputs.contains_key(kind))
        {
            return Err(MissingInput);
        }
        // Only bounded identity metadata is serialized, never data/source payloads.
        // Sorting edges makes host enumeration order irrelevant; aliases stay explicit.
        let mut canonical = self.clone();
        canonical.imports.sort();
        let bytes = serde_json::to_vec(&canonical).map_err(|_| InvalidContext)?;
        if bytes.len() > MAX_DEPENDENCY_IDENTITY_BYTES {
            return Err(Bounds);
        }
        let mut hasher = blake3::Hasher::new_derive_key("cem native dependency identity/1");
        hasher.update(&bytes);
        Ok(DependencyIdentity {
            layer: self.layer,
            hash: ContentHash {
                scheme: HASH_SCHEME.into(),
                hex: hasher.finalize().to_hex().to_string(),
            },
        })
    }
}

/// Separate, nonportable publication guard. A portable identity can survive a
/// restart; a retained handle, pending result or consumer lease cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyAdmission {
    pub owner: String,
    pub runtime_generation: u64,
    pub revision: u64,
}
impl DependencyAdmission {
    pub fn is_current(&self, current: &Self) -> bool {
        !self.owner.is_empty() && self == current
    }
}
