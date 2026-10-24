//! Opt-in immutable schema asset packs. This prototype leaves embedded sources
//! and the existing native/WASM ABI unchanged; it does not establish WASM savings.

use crate::{
    import::{import_bytes_with_lexical_scopes, import_data_bytes, MAX_DOCUMENT_BYTES},
    operation_control::{ExecutionScopeId, OperationControl},
    resolver::{
        ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolverPolicy,
        ResolverRegistry,
    },
    schema::{
        package_sources::{
            builtin_schema_package_artifact_sources, builtin_schema_package_sources,
        },
        registry::{
            builtin_schema_descriptors, content_type_essence,
            schema_descriptor_from_manifest_and_schema_documents, NamespaceClaim,
            SchemaContentType, SchemaContentTypeRole, SchemaDescriptor, SchemaRegistry,
            CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_CONTENT_TYPE, CEM_TRANSFORM_CONTENT_TYPE,
        },
        vocab::CompiledSchema,
    },
    source_map::SourceMapStack,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const SCHEMA_ASSET_PACK_FORMAT: &str = "cem-schema-asset-pack/1";

/// Named control DTOs, never an AST or a grant to activate a schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SchemaAssetPackIndex {
    pub format_version: String,
    pub packages: Vec<SchemaAssetPackageDescriptor>,
    pub sources: Vec<SchemaAssetSourceDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SchemaAssetContentType {
    pub value: String,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SchemaAssetNamespace {
    pub prefix: Option<String>,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SchemaAssetPackageDescriptor {
    pub package_id: String,
    pub schema_uri: String,
    pub version: String,
    pub source: String,
    pub content_types: Vec<SchemaAssetContentType>,
    pub namespaces: Vec<SchemaAssetNamespace>,
    pub uses: Vec<String>,
}

impl From<SchemaDescriptor> for SchemaAssetPackageDescriptor {
    fn from(value: SchemaDescriptor) -> Self {
        Self {
            package_id: value.package_id,
            schema_uri: value.schema_uri,
            version: value.version,
            source: value.source,
            content_types: value
                .content_types
                .into_iter()
                .map(|item| SchemaAssetContentType {
                    value: item.value,
                    primary: item.role == SchemaContentTypeRole::Primary,
                })
                .collect(),
            namespaces: value
                .namespaces
                .into_iter()
                .map(|item| SchemaAssetNamespace {
                    prefix: item.prefix,
                    uri: item.uri,
                })
                .collect(),
            uses: value.uses,
        }
    }
}

impl SchemaAssetPackageDescriptor {
    pub fn schema_descriptor(&self) -> SchemaDescriptor {
        SchemaDescriptor {
            package_id: self.package_id.clone(),
            schema_uri: self.schema_uri.clone(),
            version: self.version.clone(),
            source: self.source.clone(),
            content_types: self
                .content_types
                .iter()
                .map(|item| {
                    SchemaContentType::new(
                        &item.value,
                        if item.primary {
                            SchemaContentTypeRole::Primary
                        } else {
                            SchemaContentTypeRole::Alias
                        },
                    )
                })
                .collect(),
            namespaces: self
                .namespaces
                .iter()
                .map(|item| NamespaceClaim {
                    prefix: item.prefix.clone(),
                    uri: item.uri.clone(),
                })
                .collect(),
            uses: self.uses.clone(),
        }
    }

    pub(crate) fn directory(&self) -> Result<String, SchemaAssetPackError> {
        let major = self.version.split('.').next().unwrap_or("");
        if !identifier(&self.package_id)
            || self.version.len() > 64
            || major.is_empty()
            || !major.bytes().all(|byte| byte.is_ascii_digit())
            || self
                .version
                .bytes()
                .any(|byte| !(byte.is_ascii_alphanumeric() || b".-+".contains(&byte)))
        {
            return Err(error("identity", "Invalid package/version identity."));
        }
        Ok(format!("schema-packages/{}/v{major}/", self.package_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaAssetSourceKind {
    Manifest,
    Schema,
    Formatter,
    Colorizer,
    Artifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SchemaAssetSourceDescriptor {
    pub package_id: String,
    pub version: String,
    /// Original native catalog source address, independent of transport location.
    pub path: String,
    pub kind: SchemaAssetSourceKind,
    pub content_type: String,
    pub byte_length: usize,
    /// Lowercase SHA-256 hex of the exact UTF-8 source bytes.
    pub digest: String,
}

impl SchemaAssetSourceDescriptor {
    pub fn asset_path(&self) -> String {
        format!("sources/{}", self.digest)
    }
}

impl SchemaAssetPackIndex {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SchemaAssetPackError> {
        serde_json::to_vec(self).map_err(|_| error("index", "Cannot encode asset index."))
    }
}

pub struct SchemaAssetPackExportSource {
    pub descriptor: SchemaAssetSourceDescriptor,
    pub source: &'static str,
}

pub struct SchemaAssetPackExport {
    pub index: SchemaAssetPackIndex,
    pub sources: Vec<SchemaAssetPackExportSource>,
}

pub fn schema_asset_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Generate the complete bootstrap and raw-source inventory natively. Release
/// catalogs are not pruned. Identical bytes may share a digest-addressed file.
pub fn builtin_schema_asset_pack_export() -> Result<SchemaAssetPackExport, SchemaAssetPackError> {
    let mut packages: Vec<SchemaAssetPackageDescriptor> = builtin_schema_descriptors()
        .into_iter()
        .map(Into::into)
        .collect();
    packages.sort_by(|a, b| a.package_id.cmp(&b.package_id));
    let mut sources = BTreeMap::<String, SchemaAssetPackExportSource>::new();
    let mut add = |package_id: &str,
                   path: &str,
                   kind,
                   source: &'static str|
     -> Result<(), SchemaAssetPackError> {
        let package = packages
            .iter()
            .find(|package| package.package_id == package_id)
            .ok_or_else(|| error("identity", "Source has no package identity."))?;
        let content_type = match kind {
            SchemaAssetSourceKind::Manifest => CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            SchemaAssetSourceKind::Schema => CEM_SCHEMA_CONTENT_TYPE,
            _ if path.ends_with(".schema.json") => "application/schema+json",
            _ if path.ends_with(".json") => "application/json",
            _ => CEM_TRANSFORM_CONTENT_TYPE,
        };
        let descriptor = SchemaAssetSourceDescriptor {
            package_id: package_id.into(),
            version: package.version.clone(),
            path: path.into(),
            kind,
            content_type: content_type.into(),
            byte_length: source.len(),
            digest: schema_asset_sha256(source.as_bytes()),
        };
        if let Some(previous) = sources.get(path) {
            if previous.descriptor != descriptor || previous.source != source {
                return Err(error("collision", "Catalog source identities collide."));
            }
        } else {
            sources.insert(
                path.into(),
                SchemaAssetPackExportSource { descriptor, source },
            );
        }
        Ok(())
    };
    for package in builtin_schema_package_sources() {
        let parent = package
            .schema_path
            .rsplit_once("/schema/")
            .ok_or_else(|| error("identity", "Catalog schema path is invalid."))?
            .0;
        add(
            package.package_id,
            &format!("{parent}/package.cem"),
            SchemaAssetSourceKind::Manifest,
            package.manifest_source,
        )?;
        add(
            package.package_id,
            package.schema_path,
            SchemaAssetSourceKind::Schema,
            package.schema_source,
        )?;
    }
    for artifact in builtin_schema_package_artifact_sources() {
        let kind = if artifact.path.contains("/formatters/") {
            SchemaAssetSourceKind::Formatter
        } else if artifact.path.contains("/colorizers/") {
            SchemaAssetSourceKind::Colorizer
        } else {
            SchemaAssetSourceKind::Artifact
        };
        add(artifact.package_id, artifact.path, kind, artifact.source)?;
    }
    let sources: Vec<_> = sources.into_values().collect();
    let index = SchemaAssetPackIndex {
        format_version: SCHEMA_ASSET_PACK_FORMAT.into(),
        packages,
        sources: sources.iter().map(|item| item.descriptor.clone()).collect(),
    };
    // The export obeys the same checks as externally delivered bytes.
    let encoded = index.to_bytes()?;
    SchemaAssetPack::from_index_bytes(
        &encoded,
        &schema_asset_sha256(&encoded),
        SchemaAssetPackLimits::default(),
    )?;
    Ok(SchemaAssetPackExport { index, sources })
}

#[derive(Debug, Clone, Copy)]
pub struct SchemaAssetPackLimits {
    pub max_index_bytes: usize,
    pub max_sources: usize,
    pub max_entry_bytes: usize,
    pub max_total_bytes: usize,
}

impl Default for SchemaAssetPackLimits {
    fn default() -> Self {
        Self {
            max_index_bytes: 4 * 1024 * 1024,
            max_sources: 1024,
            max_entry_bytes: MAX_DOCUMENT_BYTES,
            max_total_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaAssetPackError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for SchemaAssetPackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for SchemaAssetPackError {}
fn error(kind: &'static str, message: &str) -> SchemaAssetPackError {
    let code = match kind {
        "digest" => "cem.schema_asset.digest",
        "limit" => "cem.schema_asset.limit",
        "identity" => "cem.schema_asset.identity",
        "collision" => "cem.schema_asset.collision",
        "missing" => "cem.schema_asset.missing",
        "mime" => "cem.schema_asset.mime",
        "utf8" => "cem.schema_asset.utf8",
        "cancelled" => "cem.schema_asset.cancelled",
        "policy" => "cem.schema_asset.policy",
        "transport" => "cem.schema_asset.transport",
        "import" => "cem.schema_asset.import",
        _ => "cem.schema_asset.index",
    };
    SchemaAssetPackError {
        code,
        message: message.into(),
    }
}

/// Owned per consumer/generation. This is a source store, not a schema grant,
/// checkpoint, global cache, or transport retry/fallback mechanism.
pub struct SchemaAssetPack {
    index: SchemaAssetPackIndex,
    index_digest: String,
    sources: BTreeMap<String, Arc<[u8]>>,
    transports: BTreeMap<String, String>,
    admitted_bytes: usize,
    limits: SchemaAssetPackLimits,
}

impl SchemaAssetPack {
    /// The caller pins the exact index bytes independently. Verify that pin
    /// before decoding any control DTO or interpreting source addresses.
    pub fn from_index_bytes(
        bytes: &[u8],
        expected_digest: &str,
        limits: SchemaAssetPackLimits,
    ) -> Result<Self, SchemaAssetPackError> {
        if bytes.len() > limits.max_index_bytes {
            return Err(error("limit", "Asset index exceeds its byte limit."));
        }
        if !digest_valid(expected_digest) || schema_asset_sha256(bytes) != expected_digest {
            return Err(error(
                "digest",
                "Asset index does not match its pinned digest.",
            ));
        }
        let index: SchemaAssetPackIndex = serde_json::from_slice(bytes)
            .map_err(|_| error("index", "Invalid asset index control document."))?;
        validate_index(&index, limits)?;
        Ok(Self {
            index,
            index_digest: expected_digest.into(),
            sources: BTreeMap::new(),
            transports: BTreeMap::new(),
            admitted_bytes: 0,
            limits,
        })
    }
    pub fn index(&self) -> &SchemaAssetPackIndex {
        &self.index
    }
    pub fn index_digest(&self) -> &str {
        &self.index_digest
    }
    pub fn admitted_sources(&self) -> usize {
        self.sources.len()
    }
    pub fn admitted_bytes(&self) -> usize {
        self.admitted_bytes
    }
    pub fn schema_registry(&self) -> Result<SchemaRegistry, SchemaAssetPackError> {
        registry_from_index(&self.index)
    }
    pub fn source_descriptor(
        &self,
        package_id: &str,
        version: &str,
        path: &str,
    ) -> Result<&SchemaAssetSourceDescriptor, SchemaAssetPackError> {
        self.index
            .sources
            .iter()
            .find(|source| {
                source.package_id == package_id && source.version == version && source.path == path
            })
            .ok_or_else(|| {
                error(
                    "missing",
                    "Requested source identity is absent from the pinned index.",
                )
            })
    }
    pub fn source_provenance(
        &self,
        package_id: &str,
        version: &str,
        path: &str,
    ) -> Result<SchemaAssetSourceProvenance<'_>, SchemaAssetPackError> {
        let descriptor = self.source_descriptor(package_id, version, path)?;
        self.source_bytes(package_id, version, path)?;
        Ok(SchemaAssetSourceProvenance {
            canonical_uri: &descriptor.path,
            digest: &descriptor.digest,
            index_digest: &self.index_digest,
            transport_uri: self.transports.get(path).map(String::as_str),
        })
    }
    pub fn source_request(
        &self,
        package_id: &str,
        version: &str,
        path: &str,
        base_url: &str,
        policy: &ResolverPolicy,
        source_map: SourceMapStack,
    ) -> Result<SchemaAssetSourceRequest, SchemaAssetPackError> {
        let descriptor = self.source_descriptor(package_id, version, path)?.clone();
        let base = url::Url::parse(base_url)
            .map_err(|_| error("identity", "Invalid asset transport base."))?;
        if base.cannot_be_a_base()
            || base.fragment().is_some()
            || base.query().is_some()
            || !base.path().ends_with('/')
        {
            return Err(error(
                "identity",
                "Asset transport base must be a fragment-free directory URL.",
            ));
        }
        let uri = base
            .join(&descriptor.asset_path())
            .map_err(|_| error("identity", "Invalid asset transport address."))?;
        let mut request = ResolveRequest::new(
            uri.as_str(),
            ResolvePurpose::Template,
            ResolveDirection::Read,
        )
        .with_base_uri(base.as_str())
        .with_content_type_hint(&descriptor.content_type);
        let decision = policy
            .decide(&request)
            .map_err(|_| error("policy", "Asset transport is denied by resolver policy."))?;
        let effective = url::Url::parse(&decision.effective_uri)
            .map_err(|_| error("identity", "Invalid asset policy address."))?;
        if effective.fragment().is_some() {
            return Err(error("identity", "Asset transport must be fragment-free."));
        }
        request.uri = effective.into();
        Ok(SchemaAssetSourceRequest {
            index_digest: self.index_digest.clone(),
            descriptor,
            request,
            source_map,
        })
    }
    pub fn admit(
        &mut self,
        request: &SchemaAssetSourceRequest,
        response: ResolvedRead,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<(), SchemaAssetPackError> {
        active(control, scope)?;
        if request.index_digest != self.index_digest
            || self.source_descriptor(
                &request.descriptor.package_id,
                &request.descriptor.version,
                &request.descriptor.path,
            )? != &request.descriptor
        {
            return Err(error(
                "identity",
                "Source request belongs to a different pinned index.",
            ));
        }
        // Explicit substitutions are applied before acquisition; an unrequested
        // response/redirect cannot silently rename the immutable resource.
        if response.uri != request.request.uri {
            return Err(error(
                "identity",
                "Asset response location differs from its admitted request.",
            ));
        }
        let is_new = !self.sources.contains_key(&request.descriptor.path);
        self.admit_bytes(
            &request.descriptor,
            response.bytes,
            response.content_type.as_deref(),
            control,
            scope,
        )?;
        if is_new {
            self.transports
                .insert(request.descriptor.path.clone(), response.uri);
        }
        Ok(())
    }
    /// Native callers which already obtained raw bytes use the same pinned
    /// admission checks. This never performs a read or falls back to embedding.
    pub fn admit_source(
        &mut self,
        package_id: &str,
        version: &str,
        path: &str,
        bytes: Vec<u8>,
        content_type: &str,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<(), SchemaAssetPackError> {
        let descriptor = self.source_descriptor(package_id, version, path)?.clone();
        self.admit_bytes(&descriptor, bytes, Some(content_type), control, scope)
    }
    fn admit_bytes(
        &mut self,
        descriptor: &SchemaAssetSourceDescriptor,
        bytes: Vec<u8>,
        mime: Option<&str>,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<(), SchemaAssetPackError> {
        active(control, scope)?;
        if bytes.len() != descriptor.byte_length || bytes.len() > self.limits.max_entry_bytes {
            return Err(error(
                "limit",
                "Asset source length differs from its bounded index entry.",
            ));
        }
        let is_new = !self.sources.contains_key(&descriptor.path);
        if is_new
            && self
                .admitted_bytes
                .checked_add(bytes.len())
                .filter(|total| *total <= self.limits.max_total_bytes)
                .is_none()
        {
            return Err(error(
                "limit",
                "Asset source store exceeds its aggregate byte limit.",
            ));
        }
        let mime =
            mime.ok_or_else(|| error("mime", "Asset response content type is unavailable."))?;
        if content_type_essence(mime) != descriptor.content_type || !utf8_mime(mime) {
            return Err(error(
                "mime",
                "Asset response content type differs from its pinned identity.",
            ));
        }
        if std::str::from_utf8(&bytes).is_err() {
            return Err(error("utf8", "Asset source is not UTF-8."));
        }
        if schema_asset_sha256(&bytes) != descriptor.digest {
            return Err(error(
                "digest",
                "Asset source does not match its pinned digest.",
            ));
        }
        active(control, scope)?;
        if let Some(previous) = self.sources.get(&descriptor.path) {
            if previous.as_ref() != bytes {
                return Err(error(
                    "collision",
                    "An immutable admitted source cannot be replaced.",
                ));
            }
        } else {
            self.admitted_bytes += bytes.len();
            self.sources.insert(descriptor.path.clone(), bytes.into());
        }
        Ok(())
    }
    pub fn source_bytes(
        &self,
        package_id: &str,
        version: &str,
        path: &str,
    ) -> Result<&[u8], SchemaAssetPackError> {
        self.source_descriptor(package_id, version, path)?;
        self.sources.get(path).map(AsRef::as_ref).ok_or_else(|| {
            error(
                "missing",
                "Asset source has not been admitted; no embedded fallback is permitted.",
            )
        })
    }
    /// Import creates a fresh native source owner at its original catalog
    /// address. No portable token can supply or reuse another owner's handles.
    pub fn import_source(
        &self,
        package_id: &str,
        version: &str,
        path: &str,
    ) -> Result<SchemaAssetImportedSource, SchemaAssetPackError> {
        let descriptor = self.source_descriptor(package_id, version, path)?;
        let bytes = self.source_bytes(package_id, version, path)?;
        if descriptor.content_type.ends_with("+cem") {
            let imported = import_bytes_with_lexical_scopes(
                bytes,
                &descriptor.content_type,
                path,
                CompiledSchema::cem_core(),
            )
            .map_err(|_| {
                error(
                    "import",
                    "Asset source could not be imported at the native boundary.",
                )
            })?;
            if imported
                .tree
                .ast()
                .diagnostics
                .iter()
                .chain(imported.captured.diagnostics())
                .any(|item| item.severity.is_hard_violation())
            {
                return Err(error("import", "Asset source has invalid native syntax."));
            }
            Ok(SchemaAssetImportedSource {
                tree: imported.tree,
                captured: Some(imported.captured),
            })
        } else {
            let tree =
                import_data_bytes(bytes, &descriptor.content_type, "cem", path).map_err(|_| {
                    error(
                        "import",
                        "Asset source could not be imported at the native data boundary.",
                    )
                })?;
            Ok(SchemaAssetImportedSource {
                tree,
                captured: None,
            })
        }
    }
    /// Check the operation before and after the bounded native import; an ended
    /// operation cannot publish a newly imported owner to its caller.
    pub fn import_source_with_control(
        &self,
        package_id: &str,
        version: &str,
        path: &str,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<SchemaAssetImportedSource, SchemaAssetPackError> {
        active(control, scope)?;
        let imported = self.import_source(package_id, version, path)?;
        active(control, scope)?;
        Ok(imported)
    }
    /// Return independently owned source inputs after native package identity
    /// validation. Releasing this store cannot invalidate downstream consumers.
    pub fn owned_package(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<crate::schema::package_loader::SchemaPackage, SchemaAssetPackError> {
        let loaded = self.load_package(package_id, version)?;
        let package = self
            .index
            .packages
            .iter()
            .find(|package| package.package_id == package_id && package.version == version)
            .ok_or_else(|| error("missing", "Package is absent from pinned bootstrap."))?;
        let manifest_path = format!("{}package.cem", package.directory()?);
        let text = |path: &str| -> Result<Arc<str>, SchemaAssetPackError> {
            let bytes = self.source_bytes(package_id, version, path)?;
            let source = std::str::from_utf8(bytes)
                .map_err(|_| error("utf8", "Admitted package text is invalid."))?;
            Ok(Arc::from(source))
        };
        Ok(crate::schema::package_loader::SchemaPackage {
            descriptor: loaded.descriptor,
            manifest_source: text(&manifest_path)?.into(),
            schema_source: text(&package.source)?.into(),
            source_policy: crate::schema::package_loader::SchemaPackageSourcePolicy::Authoritative,
        })
    }

    /// A strict reader for formatter/colorizer source inputs. Every read checks
    /// the original immutable admission; no embedded body can satisfy a refusal.
    pub fn conversion_reader(&self) -> SchemaAssetConversionReader<'_> {
        SchemaAssetConversionReader { pack: self }
    }

    pub fn load_package(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<SchemaAssetLoadedPackage, SchemaAssetPackError> {
        let package = self
            .index
            .packages
            .iter()
            .find(|package| package.package_id == package_id && package.version == version)
            .ok_or_else(|| {
                error(
                    "missing",
                    "Package identity is absent from the pinned index.",
                )
            })?;
        let manifest_path = format!("{}package.cem", package.directory()?);
        let schema = self.import_source(package_id, version, &package.source)?;
        let manifest = self.import_source(package_id, version, &manifest_path)?;
        let descriptor = schema_descriptor_from_manifest_and_schema_documents(
            package_id,
            manifest.tree.ast(),
            &package.source,
            schema.tree.ast(),
        )
        .map_err(|_| error("identity", "Native package identity could not be derived."))?;
        if descriptor != package.schema_descriptor() {
            return Err(error(
                "identity",
                "Native source identity differs from the pinned bootstrap.",
            ));
        }
        Ok(SchemaAssetLoadedPackage {
            descriptor,
            manifest,
            schema,
        })
    }
}

pub struct SchemaAssetConversionReader<'a> {
    pack: &'a SchemaAssetPack,
}

impl crate::conversion::ConversionPackageArtifactSourceReader for SchemaAssetConversionReader<'_> {
    fn read(
        &self,
        artifact: &crate::conversion::ConversionPackageArtifactDescriptor,
    ) -> Result<crate::conversion::ConversionPackageArtifactRead, String> {
        let package = self
            .pack
            .index
            .packages
            .iter()
            .find(|package| package.package_id == artifact.package_id)
            .ok_or_else(|| {
                error(
                    "missing",
                    "Artifact package is absent from pinned bootstrap.",
                )
                .to_string()
            })?;
        let descriptor = self
            .pack
            .source_descriptor(&artifact.package_id, &package.version, &artifact.path)
            .map_err(|error| error.to_string())?;
        let provenance = self
            .pack
            .source_provenance(&artifact.package_id, &package.version, &artifact.path)
            .map_err(|error| error.to_string())?;
        Ok(crate::conversion::ConversionPackageArtifactRead {
            uri: provenance.canonical_uri.to_owned(),
            bytes: self
                .pack
                .source_bytes(&artifact.package_id, &package.version, &artifact.path)
                .map_err(|error| error.to_string())?
                .to_vec(),
            content_type: Some(descriptor.content_type.clone()),
        })
    }
    fn allow_embedded_fallback(&self) -> bool {
        false
    }
}

pub struct SchemaAssetLoadedPackage {
    pub descriptor: SchemaDescriptor,
    pub manifest: SchemaAssetImportedSource,
    pub schema: SchemaAssetImportedSource,
}

/// CEM sources retain lexical associations with their exact original arena.
/// Non-CEM data artifacts retain their native import owner and have no CEM
/// lexical references; no synthetic bindings or host document objects are made.
pub struct SchemaAssetImportedSource {
    pub tree: Arc<crate::parser::tree::RetainedCemTree>,
    pub captured: Option<Arc<crate::schema::machine::LexicallyScopedDocument>>,
}

/// Canonical source provenance survives transport relocation. Reimport uses the
/// original catalog address and exact bytes; each new store owns fresh arenas.
pub struct SchemaAssetSourceProvenance<'a> {
    pub canonical_uri: &'a str,
    pub digest: &'a str,
    pub index_digest: &'a str,
    pub transport_uri: Option<&'a str>,
}

pub struct SchemaAssetSourceRequest {
    index_digest: String,
    descriptor: SchemaAssetSourceDescriptor,
    request: ResolveRequest,
    source_map: SourceMapStack,
}
impl SchemaAssetSourceRequest {
    pub fn request(&self) -> &ResolveRequest {
        &self.request
    }
    pub fn descriptor(&self) -> &SchemaAssetSourceDescriptor {
        &self.descriptor
    }
    pub fn source_map(&self) -> &SourceMapStack {
        &self.source_map
    }
    pub fn read(
        &self,
        registry: &ResolverRegistry,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<ResolvedRead, SchemaAssetPackError> {
        active(control, scope)?;
        let response = registry.read_with_control(&self.request, control, scope);
        active(control, scope)?;
        response.map_err(|_| {
            error(
                "transport",
                "Asset transport failed or was cancelled; no embedded fallback is permitted.",
            )
        })
    }
}

fn active(control: &OperationControl, scope: ExecutionScopeId) -> Result<(), SchemaAssetPackError> {
    control
        .check_scope(scope)
        .map_err(|_| error("cancelled", "Asset operation is no longer active."))
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn path_valid(value: &str) -> bool {
    value.len() <= 2048
        && !value.starts_with('/')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/-_.".contains(&byte))
}
fn utf8_mime(value: &str) -> bool {
    value.split(';').skip(1).all(|parameter| {
        parameter
            .trim()
            .split_once('=')
            .is_some_and(|(key, value)| {
                key.trim().eq_ignore_ascii_case("charset")
                    && value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
            })
    })
}
fn registry_from_index(
    index: &SchemaAssetPackIndex,
) -> Result<SchemaRegistry, SchemaAssetPackError> {
    let mut registry = SchemaRegistry::new();
    for package in &index.packages {
        registry
            .register(package.schema_descriptor())
            .map_err(|_| error("collision", "Asset bootstrap schema identities conflict."))?;
    }
    Ok(registry)
}
fn validate_index(
    index: &SchemaAssetPackIndex,
    limits: SchemaAssetPackLimits,
) -> Result<(), SchemaAssetPackError> {
    if index.format_version != SCHEMA_ASSET_PACK_FORMAT {
        return Err(error("index", "Unsupported asset index version."));
    }
    if index.sources.len() > limits.max_sources
        || index.packages.len() > limits.max_sources
        || index.packages.is_empty()
    {
        return Err(error(
            "limit",
            "Asset index exceeds its source-count limit.",
        ));
    }
    let mut packages = BTreeMap::new();
    for package in &index.packages {
        let directory = package.directory()?;
        if !path_valid(&package.source)
            || !package.source.starts_with(&directory)
            || url::Url::parse(&package.schema_uri).is_err()
            || package.schema_uri.len() > 2048
        {
            return Err(error(
                "identity",
                "Invalid asset bootstrap schema identity.",
            ));
        }
        if packages
            .insert(package.package_id.as_str(), package)
            .is_some()
        {
            return Err(error("collision", "Duplicate asset package identity."));
        }
        if package.content_types.is_empty()
            || package.content_types.len() > 128
            || package.namespaces.len() > 128
            || package.uses.len() > 128
        {
            return Err(error(
                "limit",
                "Asset bootstrap metadata exceeds its bound.",
            ));
        }
        let mut content_types = BTreeSet::new();
        for item in &package.content_types {
            // Native package aliases include symbolic identities such as
            // custom-element-xslt; retain them rather than inventing MIME-only
            // registry rules at the asset transport boundary.
            if item.value.is_empty()
                || item.value.len() > 256
                || item.value.bytes().any(|byte| byte.is_ascii_control())
                || !content_types.insert(item.value.as_str())
            {
                return Err(error(
                    "identity",
                    "Invalid or duplicate bootstrap content type.",
                ));
            }
        }
        if package
            .namespaces
            .iter()
            .filter_map(|item| item.prefix.as_deref())
            .any(|prefix| prefix.len() > 256 || prefix.bytes().any(|byte| byte.is_ascii_control()))
        {
            return Err(error(
                "identity",
                "Invalid or oversized bootstrap namespace prefix.",
            ));
        }
        for uri in package
            .namespaces
            .iter()
            .map(|item| &item.uri)
            .chain(&package.uses)
        {
            if uri.len() > 2048 || url::Url::parse(uri).is_err() {
                return Err(error(
                    "identity",
                    "Invalid bootstrap namespace/dependency URI.",
                ));
            }
        }
    }
    let mut paths = BTreeSet::new();
    let mut digests = BTreeMap::new();
    for source in &index.sources {
        let package = packages
            .get(source.package_id.as_str())
            .ok_or_else(|| error("identity", "Asset source has no bootstrap package."))?;
        if source.version != package.version
            || !path_valid(&source.path)
            || !source.path.starts_with(&package.directory()?)
            || !digest_valid(&source.digest)
        {
            return Err(error("identity", "Invalid asset source identity."));
        }
        if !paths.insert(source.path.as_str()) {
            return Err(error("collision", "Duplicate asset source path."));
        }
        if source.byte_length > limits.max_entry_bytes || source.byte_length > MAX_DOCUMENT_BYTES {
            return Err(error(
                "limit",
                "Asset source exceeds its individual byte limit.",
            ));
        }
        if let Some(length) = digests.insert(&source.digest, source.byte_length) {
            if length != source.byte_length {
                return Err(error(
                    "collision",
                    "One digest names conflicting source lengths.",
                ));
            }
        }
        let expected_mime = match source.kind {
            SchemaAssetSourceKind::Manifest => CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            SchemaAssetSourceKind::Schema => CEM_SCHEMA_CONTENT_TYPE,
            _ if source.path.ends_with(".schema.json") => "application/schema+json",
            _ if source.path.ends_with(".json") => "application/json",
            _ => CEM_TRANSFORM_CONTENT_TYPE,
        };
        if source.content_type != expected_mime {
            return Err(error("mime", "Unsupported asset source content type."));
        }
        if source.kind == SchemaAssetSourceKind::Manifest
            && source.path != format!("{}package.cem", package.directory()?)
            || source.kind == SchemaAssetSourceKind::Schema && source.path != package.source
        {
            return Err(error(
                "identity",
                "Asset manifest/schema source path differs from its bootstrap.",
            ));
        }
    }
    for package in index.packages.iter() {
        for (path, kind) in [
            (
                format!("{}package.cem", package.directory()?),
                SchemaAssetSourceKind::Manifest,
            ),
            (package.source.clone(), SchemaAssetSourceKind::Schema),
        ] {
            if !index.sources.iter().any(|source| {
                source.package_id == package.package_id
                    && source.path == path
                    && source.kind == kind
            }) {
                return Err(error(
                    "missing",
                    "Asset bootstrap package lacks its manifest or schema entry.",
                ));
            }
        }
    }
    registry_from_index(index)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        operation_control::OperationControl, resolver::ResolvedRead,
        schema::registry::builtin_schema_descriptors, source_map::SourceMapStack,
    };

    fn fixture() -> &'static SchemaAssetPackExport {
        static FIXTURE: std::sync::OnceLock<SchemaAssetPackExport> = std::sync::OnceLock::new();
        FIXTURE.get_or_init(|| builtin_schema_asset_pack_export().unwrap())
    }
    fn store(
        index: &SchemaAssetPackIndex,
        limits: SchemaAssetPackLimits,
    ) -> Result<SchemaAssetPack, SchemaAssetPackError> {
        let bytes = index.to_bytes().unwrap();
        SchemaAssetPack::from_index_bytes(&bytes, &schema_asset_sha256(&bytes), limits)
    }
    fn admit(
        pack: &mut SchemaAssetPack,
        source: &SchemaAssetPackExportSource,
    ) -> Result<(), SchemaAssetPackError> {
        let control = OperationControl::default();
        pack.admit_source(
            &source.descriptor.package_id,
            &source.descriptor.version,
            &source.descriptor.path,
            source.source.as_bytes().to_vec(),
            &source.descriptor.content_type,
            &control,
            control.root_scope(),
        )
    }

    #[test]
    fn deterministic_bootstrap_retains_all_native_identities_and_sources() {
        let first = builtin_schema_asset_pack_export().unwrap();
        let second = builtin_schema_asset_pack_export().unwrap();
        assert_eq!(
            first.index.to_bytes().unwrap(),
            second.index.to_bytes().unwrap()
        );
        assert_eq!(
            first.index.packages.len(),
            builtin_schema_descriptors().len()
        );
        let bytes = first.index.to_bytes().unwrap();
        let pack = SchemaAssetPack::from_index_bytes(
            &bytes,
            &schema_asset_sha256(&bytes),
            SchemaAssetPackLimits::default(),
        )
        .unwrap();
        for descriptor in builtin_schema_descriptors() {
            assert_eq!(
                pack.schema_registry()
                    .unwrap()
                    .schema(&descriptor.schema_uri),
                Some(&descriptor)
            );
        }
        assert_eq!(pack.admitted_sources(), 0);
        assert_eq!(pack.admitted_bytes(), 0);
        assert_eq!(pack.index().sources.len(), first.sources.len());
        assert!(first
            .sources
            .iter()
            .all(|source| source.descriptor.byte_length == source.source.len()));
    }

    #[test]
    fn compiled_bootstrap_pin_matches_complete_native_catalog() {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "camelCase")]
        struct Bootstrap {
            index_digest: String,
            index_bytes: usize,
            package_count: usize,
            source_count: usize,
            source_bytes: usize,
        }
        let compiled: Bootstrap = serde_json::from_slice(include_bytes!(
            "../../schema/schema-assets-bootstrap-v1.json"
        ))
        .unwrap();
        let export = fixture();
        let index = export.index.to_bytes().unwrap();
        assert_eq!(compiled.index_digest, schema_asset_sha256(&index));
        assert_eq!(compiled.index_bytes, index.len());
        assert_eq!(compiled.package_count, export.index.packages.len());
        assert_eq!(compiled.source_count, export.sources.len());
        assert_eq!(
            compiled.source_bytes,
            export
                .sources
                .iter()
                .map(|source| source.descriptor.byte_length)
                .sum::<usize>()
        );
    }

    #[test]
    fn source_admission_is_lazy_pinned_bounded_and_reimports_fresh_owners() {
        let export = builtin_schema_asset_pack_export().unwrap();
        let bytes = export.index.to_bytes().unwrap();
        let digest = schema_asset_sha256(&bytes);
        let mut pack =
            SchemaAssetPack::from_index_bytes(&bytes, &digest, SchemaAssetPackLimits::default())
                .unwrap();
        let source = export
            .sources
            .iter()
            .find(|source| {
                source.descriptor.package_id == "json"
                    && source.descriptor.kind == SchemaAssetSourceKind::Schema
            })
            .unwrap();
        let request = pack
            .source_request(
                &source.descriptor.package_id,
                &source.descriptor.version,
                &source.descriptor.path,
                "https://assets.test/pack/",
                &crate::resolver::ResolverPolicy::new(),
                SourceMapStack::default(),
            )
            .unwrap();
        let control = OperationControl::default();
        pack.admit(
            &request,
            ResolvedRead {
                uri: request.request().uri.clone(),
                bytes: source.source.as_bytes().to_vec(),
                content_type: Some(source.descriptor.content_type.clone()),
            },
            &control,
            control.root_scope(),
        )
        .unwrap();
        assert_eq!(pack.admitted_sources(), 1);
        assert_eq!(pack.admitted_bytes(), source.source.len());
        let first = pack
            .import_source("json", &source.descriptor.version, &source.descriptor.path)
            .unwrap();
        let mut fresh =
            SchemaAssetPack::from_index_bytes(&bytes, &digest, SchemaAssetPackLimits::default())
                .unwrap();
        fresh
            .admit(
                &request,
                ResolvedRead {
                    uri: request.request().uri.clone(),
                    bytes: source.source.as_bytes().to_vec(),
                    content_type: Some(source.descriptor.content_type.clone()),
                },
                &control,
                control.root_scope(),
            )
            .unwrap();
        let second = fresh
            .import_source("json", &source.descriptor.version, &source.descriptor.path)
            .unwrap();
        assert_eq!(first.tree.source_uri(), source.descriptor.path);
        assert_eq!(second.tree.source_uri(), source.descriptor.path);
        assert!(!std::sync::Arc::ptr_eq(
            first.tree.ast_owner(),
            second.tree.ast_owner()
        ));
        assert!(std::sync::Arc::ptr_eq(
            first.tree.ast_owner(),
            first.captured.as_ref().unwrap().document()
        ));
        admit(&mut pack, source).unwrap();
        assert_eq!(pack.admitted_sources(), 1);
        assert_eq!(pack.admitted_bytes(), source.source.len());
    }

    #[test]
    fn index_pin_is_verified_before_control_decode_and_unknown_fields_are_refused() {
        let malformed = b"not a control document";
        assert_eq!(
            SchemaAssetPack::from_index_bytes(
                malformed,
                &"0".repeat(64),
                SchemaAssetPackLimits::default()
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.digest"
        );
        assert_eq!(
            SchemaAssetPack::from_index_bytes(
                malformed,
                &schema_asset_sha256(malformed),
                SchemaAssetPackLimits::default()
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.index"
        );
        let bytes = fixture().index.to_bytes().unwrap();
        let with_extra = format!(
            "{{\"unrecognized\":true,{}",
            std::str::from_utf8(&bytes).unwrap().trim_start_matches('{')
        );
        assert_eq!(
            SchemaAssetPack::from_index_bytes(
                with_extra.as_bytes(),
                &schema_asset_sha256(with_extra.as_bytes()),
                SchemaAssetPackLimits::default()
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.index"
        );
        assert_eq!(
            SchemaAssetPack::from_index_bytes(
                &bytes,
                &schema_asset_sha256(&bytes),
                SchemaAssetPackLimits {
                    max_index_bytes: bytes.len() - 1,
                    ..SchemaAssetPackLimits::default()
                }
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.limit"
        );
    }

    #[test]
    fn index_refuses_collisions_escaped_paths_wrong_versions_mimes_and_limits() {
        let base = &fixture().index;
        let mut variants = Vec::new();
        let mut index = base.clone();
        index.sources.push(index.sources[0].clone());
        variants.push((index, "collision"));
        let mut index = base.clone();
        index.packages.push(index.packages[0].clone());
        variants.push((index, "collision"));
        let mut index = base.clone();
        index.sources[0].path = "schema-packages/../escape.cem".into();
        variants.push((index, "identity"));
        let mut index = base.clone();
        index.sources[0].path = "schema-packages/alien/v1/package.cem".into();
        variants.push((index, "identity"));
        let mut index = base.clone();
        index.sources[0].version = "2.0.0".into();
        variants.push((index, "identity"));
        let mut index = base.clone();
        index.sources[0].content_type = "text/javascript".into();
        variants.push((index, "mime"));
        let mut index = base.clone();
        index.sources[0].digest = "f".repeat(63);
        variants.push((index, "identity"));
        let mut index = base.clone();
        index.sources[0].byte_length = MAX_DOCUMENT_BYTES + 1;
        variants.push((index, "limit"));
        let mut index = base.clone();
        index.sources.remove(0);
        variants.push((index, "missing"));
        let mut index = base.clone();
        index.sources[1].digest = index.sources[0].digest.clone();
        index.sources[1].byte_length = index.sources[0].byte_length + 1;
        variants.push((index, "collision"));
        for (index, suffix) in variants {
            assert_eq!(
                store(&index, SchemaAssetPackLimits::default())
                    .err()
                    .unwrap()
                    .code,
                format!("cem.schema_asset.{suffix}")
            );
        }
        assert_eq!(
            store(
                base,
                SchemaAssetPackLimits {
                    max_sources: 1,
                    ..SchemaAssetPackLimits::default()
                }
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.limit"
        );
        assert_eq!(
            store(
                base,
                SchemaAssetPackLimits {
                    max_entry_bytes: 1,
                    ..SchemaAssetPackLimits::default()
                }
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.limit"
        );
    }

    #[test]
    fn bootstrap_namespace_prefixes_are_bounded_without_changing_native_names() {
        for prefix in [
            "x".repeat(257),
            "has\ncontrol".into(),
            "has\0control".into(),
        ] {
            let mut index = fixture().index.clone();
            index.packages[0].namespaces.push(SchemaAssetNamespace {
                prefix: Some(prefix),
                uri: "https://vendor.test/namespace".into(),
            });
            assert_eq!(
                store(&index, SchemaAssetPackLimits::default())
                    .err()
                    .unwrap()
                    .code,
                "cem.schema_asset.identity"
            );
        }
        let mut index = fixture().index.clone();
        index.packages[0].namespaces.push(SchemaAssetNamespace {
            prefix: Some(String::new()),
            uri: "https://vendor.test/default".into(),
        });
        store(&index, SchemaAssetPackLimits::default()).unwrap();
    }

    #[test]
    fn source_failure_never_mutates_store_or_falls_back_to_embedding() {
        let export = fixture();
        let source = &export.sources[0];
        let descriptor = &source.descriptor;
        let mut pack = store(&export.index, SchemaAssetPackLimits::default()).unwrap();
        assert_eq!(
            pack.import_source(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.missing"
        );
        assert_eq!(
            pack.source_descriptor(&descriptor.package_id, "unknown", &descriptor.path)
                .err()
                .unwrap()
                .code,
            "cem.schema_asset.missing"
        );
        let mut bad_digest = source.source.as_bytes().to_vec();
        bad_digest[0] ^= 1;
        let mut bad_utf8 = source.source.as_bytes().to_vec();
        bad_utf8[0] = 0xff;
        let control = OperationControl::default();
        for (bytes, mime, suffix) in [
            (source.source.as_bytes().to_vec(), "text/javascript", "mime"),
            (
                source.source.as_bytes().to_vec(),
                "application/vnd.cem.schema-package+cem; charset=latin1",
                "mime",
            ),
            (Vec::new(), descriptor.content_type.as_str(), "limit"),
            (bad_digest, descriptor.content_type.as_str(), "digest"),
            (bad_utf8, descriptor.content_type.as_str(), "utf8"),
        ] {
            let result = pack.admit_source(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                bytes,
                mime,
                &control,
                control.root_scope(),
            );
            assert_eq!(
                result.err().unwrap().code,
                format!("cem.schema_asset.{suffix}")
            );
            assert_eq!((pack.admitted_sources(), pack.admitted_bytes()), (0, 0));
        }
        admit(&mut pack, source).unwrap();
        let mut changed = source.source.as_bytes().to_vec();
        changed[0] ^= 1;
        assert!(pack
            .admit_source(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                changed,
                &descriptor.content_type,
                &control,
                control.root_scope()
            )
            .is_err());
        assert_eq!(
            pack.source_bytes(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path
            )
            .unwrap(),
            source.source.as_bytes()
        );
    }

    #[test]
    fn aggregate_admission_is_bounded_without_eagerly_loading_catalog() {
        let export = fixture();
        let mut sources: Vec<_> = export.sources.iter().collect();
        sources.sort_by_key(|source| source.source.len());
        let source = sources[0];
        let other = sources[1];
        let mut pack = store(
            &export.index,
            SchemaAssetPackLimits {
                max_total_bytes: source.source.len(),
                ..SchemaAssetPackLimits::default()
            },
        )
        .unwrap();
        admit(&mut pack, source).unwrap();
        assert_eq!(
            admit(&mut pack, other).err().unwrap().code,
            "cem.schema_asset.limit"
        );
        assert_eq!(
            (pack.admitted_sources(), pack.admitted_bytes()),
            (1, source.source.len())
        );
    }

    #[test]
    fn complete_catalog_sources_import_natively_with_original_provenance() {
        let export = fixture();
        let mut pack = store(&export.index, SchemaAssetPackLimits::default()).unwrap();
        let mut total = 0;
        for source in &export.sources {
            admit(&mut pack, source).unwrap();
            let descriptor = &source.descriptor;
            let imported = pack
                .import_source(
                    &descriptor.package_id,
                    &descriptor.version,
                    &descriptor.path,
                )
                .unwrap_or_else(|error| panic!("{}: {}", descriptor.path, error));
            assert_eq!(imported.tree.source_uri(), descriptor.path);
            if let Some(captured) = &imported.captured {
                assert!(Arc::ptr_eq(imported.tree.ast_owner(), captured.document()));
            }
            let provenance = pack
                .source_provenance(
                    &descriptor.package_id,
                    &descriptor.version,
                    &descriptor.path,
                )
                .unwrap();
            assert_eq!(provenance.canonical_uri, descriptor.path);
            assert_eq!(provenance.digest, descriptor.digest);
            assert_eq!(provenance.index_digest, pack.index_digest());
            assert_eq!(provenance.transport_uri, None);
            total += descriptor.byte_length;
        }
        assert_eq!(pack.admitted_sources(), export.sources.len());
        assert_eq!(pack.admitted_bytes(), total);
        for package in &export.index.packages {
            assert_eq!(
                pack.load_package(&package.package_id, &package.version)
                    .unwrap()
                    .descriptor,
                package.schema_descriptor()
            );
            let owned = pack
                .owned_package(&package.package_id, &package.version)
                .unwrap();
            assert_eq!(owned.descriptor, package.schema_descriptor());
            assert!(!owned.manifest_source.is_empty());
            assert!(!owned.schema_source.is_empty());
        }
    }

    #[test]
    fn package_load_revalidates_native_metadata_and_json_artifacts_keep_native_provenance() {
        let export = fixture();
        let mut pack = store(&export.index, SchemaAssetPackLimits::default()).unwrap();
        for source in export
            .sources
            .iter()
            .filter(|source| source.descriptor.package_id == "json")
        {
            admit(&mut pack, source).unwrap();
        }
        let loaded = pack.load_package("json", "1.0.0").unwrap();
        assert_eq!(
            loaded.descriptor,
            builtin_schema_descriptors()
                .into_iter()
                .find(|package| package.package_id == "json")
                .unwrap()
        );
        assert_eq!(
            loaded.manifest.tree.source_uri(),
            "schema-packages/json/v1/package.cem"
        );
        assert_eq!(
            loaded.schema.tree.source_uri(),
            "schema-packages/json/v1/schema/json.cem"
        );
        let json = export
            .sources
            .iter()
            .find(|source| source.descriptor.path.ends_with(".schema.json"))
            .unwrap();
        admit(&mut pack, json).unwrap();
        let imported = pack
            .import_source(
                &json.descriptor.package_id,
                &json.descriptor.version,
                &json.descriptor.path,
            )
            .unwrap();
        assert_eq!(imported.tree.source_uri(), json.descriptor.path);
        assert!(imported.captured.is_none());
        assert!(imported.tree.ast().nodes.len() > 10);
        let mut index = export.index.clone();
        index
            .packages
            .iter_mut()
            .find(|package| package.package_id == "json")
            .unwrap()
            .schema_uri = "https://wrong.test/schema".into();
        let mut mismatched = store(&index, SchemaAssetPackLimits::default()).unwrap();
        for source in export
            .sources
            .iter()
            .filter(|source| source.descriptor.package_id == "json")
        {
            admit(&mut mismatched, source).unwrap();
        }
        assert_eq!(
            mismatched.load_package("json", "1.0.0").err().unwrap().code,
            "cem.schema_asset.identity"
        );
    }

    #[test]
    fn resolver_policy_offline_and_cancellation_remain_authoritative() {
        use crate::resolver::{
            ResolvePolicyDenial, ResolvePolicyRequestKey, ResolvePolicySubstitution, ResolvedWrite,
            ResolverDiagnostic, ResourceResolver,
        };
        use crate::scheduler::AbortSignal;
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Reader {
            calls: Arc<AtomicUsize>,
            source: &'static str,
            mime: String,
            abort: Option<AbortSignal>,
        }
        impl ResourceResolver for Reader {
            fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                if let Some(abort) = &self.abort {
                    abort.abort();
                }
                Ok(ResolvedRead {
                    uri: request.uri.clone(),
                    bytes: self.source.as_bytes().to_vec(),
                    content_type: Some(self.mime.clone()),
                })
            }
            fn write(
                &self,
                _: &ResolveRequest,
                _: &[u8],
            ) -> Result<ResolvedWrite, ResolverDiagnostic> {
                unreachable!()
            }
        }
        let export = fixture();
        let source = &export.sources[0];
        let descriptor = &source.descriptor;
        let mut pack = store(&export.index, SchemaAssetPackLimits::default()).unwrap();
        let request = pack
            .source_request(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                "https://assets.test/pack/",
                &ResolverPolicy::new(),
                SourceMapStack::default(),
            )
            .unwrap();
        let key = ResolvePolicyRequestKey::new(
            &request.request().uri,
            ResolvePurpose::Template,
            ResolveDirection::Read,
        );
        let denied =
            ResolverPolicy::new().with_denial(key.clone(), ResolvePolicyDenial::new("denied"));
        assert_eq!(
            pack.source_request(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                "https://assets.test/pack/",
                &denied,
                SourceMapStack::default()
            )
            .err()
            .unwrap()
            .code,
            "cem.schema_asset.policy"
        );
        let control = OperationControl::default();
        assert!(request
            .read(&ResolverRegistry::new(), &control, control.root_scope())
            .is_err());
        assert_eq!(pack.admitted_sources(), 0);
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = ResolverRegistry::new();
        registry.register(
            "https",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            Reader {
                calls: calls.clone(),
                source: source.source,
                mime: descriptor.content_type.clone(),
                abort: None,
            },
        );
        let abort = AbortSignal::new();
        abort.abort();
        let cancelled = OperationControl::new(abort);
        assert!(request
            .read(&registry, &cancelled, cancelled.root_scope())
            .is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(pack
            .admit_source(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                source.source.as_bytes().to_vec(),
                &descriptor.content_type,
                &cancelled,
                cancelled.root_scope()
            )
            .is_err());
        let mirror = "https://offline.test/copied-source";
        let policy = ResolverPolicy::new().with_substitution(
            key,
            ResolvePolicySubstitution::new(mirror, "offline mirror"),
        );
        let substituted = pack
            .source_request(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                "https://assets.test/pack/",
                &policy,
                SourceMapStack::default(),
            )
            .unwrap();
        let response = substituted
            .read(&registry, &control, control.root_scope())
            .unwrap();
        assert_eq!(response.uri, mirror);
        pack.admit(&substituted, response, &control, control.root_scope())
            .unwrap();
        assert_eq!(pack.admitted_sources(), 1);
        assert_eq!(
            pack.source_provenance(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path
            )
            .unwrap()
            .transport_uri,
            Some(mirror)
        );
        let wrong = ResolvedRead {
            uri: "https://wrong.test/redirect".into(),
            bytes: source.source.as_bytes().to_vec(),
            content_type: Some(descriptor.content_type.clone()),
        };
        assert_eq!(
            pack.admit(&request, wrong, &control, control.root_scope())
                .err()
                .unwrap()
                .code,
            "cem.schema_asset.identity"
        );
        let abort = AbortSignal::new();
        let cancelled = OperationControl::new(abort.clone());
        registry.register(
            "https",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            Reader {
                calls,
                source: source.source,
                mime: descriptor.content_type.clone(),
                abort: Some(abort),
            },
        );
        assert!(request
            .read(&registry, &cancelled, cancelled.root_scope())
            .is_err());
        assert!(pack
            .import_source_with_control(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                &cancelled,
                cancelled.root_scope()
            )
            .is_err());
    }

    #[test]
    fn same_length_source_digest_and_exact_index_request_witness_are_required() {
        let export = fixture();
        let source = &export.sources[0];
        let descriptor = &source.descriptor;
        let pack = store(&export.index, SchemaAssetPackLimits::default()).unwrap();
        let request = pack
            .source_request(
                &descriptor.package_id,
                &descriptor.version,
                &descriptor.path,
                "https://assets.test/pack/",
                &ResolverPolicy::new(),
                SourceMapStack::default(),
            )
            .unwrap();
        let mut changed_index = export.index.clone();
        changed_index.packages[0]
            .uses
            .push("https://different.test/dependency".into());
        let mut other = store(&changed_index, SchemaAssetPackLimits::default()).unwrap();
        let control = OperationControl::default();
        assert_eq!(
            other
                .admit(
                    &request,
                    ResolvedRead {
                        uri: request.request().uri.clone(),
                        bytes: source.source.as_bytes().to_vec(),
                        content_type: Some(descriptor.content_type.clone())
                    },
                    &control,
                    control.root_scope()
                )
                .err()
                .unwrap()
                .code,
            "cem.schema_asset.identity"
        );
        assert_eq!(other.admitted_sources(), 0);
    }
}
