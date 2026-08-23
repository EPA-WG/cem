//! Shared lowering from parsed transform-graph configuration to engine requests.
//!
//! Graph semantics live here so command-service execution and the native CLI
//! produce the same request. Resource access remains explicit: command-service
//! callers use the manifest provider, while the native CLI uses the
//! filesystem/resolver provider.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::command_service::VirtualResourceV1;
use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{
    self, EngineContext, EngineError, EngineInput, FormatIdentity, TemplateInput,
    TransformExecutionPolicy, TransformGraphConversion, TransformGraphDependency,
    TransformGraphDependencyRole, TransformGraphExport, TransformGraphImport,
    TransformGraphImportMapMissingPolicy, TransformGraphImportMapRewrite,
    TransformGraphImportMapRewriteMode, TransformGraphJoin, TransformGraphJoinInput,
    TransformGraphJoinMode, TransformGraphModuleAsset, TransformGraphModuleAssetManifest,
    TransformGraphRequest, TransformGraphStage, TransformGraphStylePolicy, TransformRuntimePhase,
    TransformStageSchedulerScopeIds, TransformTemplateEntrypoint, TransformTemplateKind,
};
use crate::resolver::{
    is_windows_drive_path, local_file_uri_to_path, local_path_or_file_uri, uri_scheme,
    ResolveDirection, ResolveListRequest, ResolvePurpose, ResolveRequest, ResolverDiagnostic,
};
use crate::run_config::{self, ScopeConfig};
use crate::schema::registry::{
    content_type_essence, MODULE_MAP_CONTENT_TYPE, MODULE_MAP_SCHEMA_URI, MODULE_MAP_V2_SCHEMA_URI,
    MODULE_MAP_V3_SCHEMA_URI,
};
use crate::transform_config::{
    self, TransformGraphConfig, TransformGraphEdgeRole, TransformGraphJoinMode as ConfigJoinMode,
    TransformGraphNode, TransformGraphNodeKind,
};

pub const TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES: usize = 1024;

#[derive(Debug)]
pub enum TransformGraphRequestError {
    Diagnostic(Box<Diagnostic>),
    Engine(EngineError),
}

impl TransformGraphRequestError {
    pub fn diagnostic(
        uri: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Diagnostic(Box::new(Diagnostic {
            uri: Some(uri.into()),
            code: code.into(),
            severity: Severity::Fatal,
            message: message.into(),
            ..Diagnostic::default()
        }))
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Diagnostic(diagnostic) => &diagnostic.code,
            Self::Engine(EngineError::Cancelled { .. }) => "cem.operation.cancelled",
            Self::Engine(EngineError::Control(failure)) => failure.code(),
            Self::Engine(EngineError::Io { .. }) => "cem.transform_graph.resource_io",
            Self::Engine(EngineError::NotImplemented) => "cem.engine.not_implemented",
            Self::Engine(EngineError::SchemaResolution(_)) => "cem.schema.resolution",
            Self::Engine(EngineError::Internal(_)) => "cem.engine.internal",
        }
    }
}

impl std::fmt::Display for TransformGraphRequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diagnostic(diagnostic) => {
                write!(formatter, "{}: {}", diagnostic.code, diagnostic.message)
            }
            Self::Engine(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for TransformGraphRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Diagnostic(_) => None,
            Self::Engine(error) => Some(error),
        }
    }
}

impl From<EngineError> for TransformGraphRequestError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

#[derive(Debug, Clone)]
pub struct TransformGraphResource {
    pub uri: String,
    pub bytes: Vec<u8>,
    pub identity: Option<FormatIdentity>,
    pub content_type: Option<String>,
}

pub trait TransformGraphResourceProvider {
    fn import_resources(
        &self,
        reference: &str,
        content_type_hint: Option<&str>,
    ) -> Result<Vec<TransformGraphResource>, TransformGraphRequestError>;

    fn read_resource(
        &self,
        reference: &str,
        purpose: ResolvePurpose,
        content_type_hint: Option<&str>,
    ) -> Result<TransformGraphResource, TransformGraphRequestError>;

    fn read_resource_relative_to(
        &self,
        base_uri: &str,
        reference: &str,
        purpose: ResolvePurpose,
        content_type_hint: Option<&str>,
    ) -> Result<TransformGraphResource, TransformGraphRequestError> {
        let resolved = resolve_transform_graph_reference(base_uri, reference);
        self.read_resource(&resolved, purpose, content_type_hint)
    }

    fn output_uri(&self, reference: &str) -> Result<String, TransformGraphRequestError>;

    fn binding_uri(&self, uri: &str) -> String;
}

#[derive(Debug)]
pub struct ManifestTransformGraphResourceProvider<'a> {
    config_uri: &'a str,
    resources: &'a BTreeMap<String, VirtualResourceV1>,
}

impl<'a> ManifestTransformGraphResourceProvider<'a> {
    pub fn new(config_uri: &'a str, resources: &'a BTreeMap<String, VirtualResourceV1>) -> Self {
        Self {
            config_uri,
            resources,
        }
    }

    fn resolved(&self, reference: &str) -> String {
        resolve_transform_graph_reference(self.config_uri, reference)
    }

    fn resource(&self, uri: &str) -> Result<TransformGraphResource, TransformGraphRequestError> {
        let resource = self.resources.get(uri).ok_or_else(|| {
            TransformGraphRequestError::diagnostic(
                self.config_uri,
                "cem.command_service.inline_resource_required",
                format!("command-service preparation requires inline bytes for `{uri}`"),
            )
        })?;
        Ok(TransformGraphResource {
            uri: uri.to_owned(),
            bytes: resource.bytes.clone(),
            identity: resource.identity.clone(),
            content_type: resource
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.clone()),
        })
    }
}

impl TransformGraphResourceProvider for ManifestTransformGraphResourceProvider<'_> {
    fn import_resources(
        &self,
        reference: &str,
        _content_type_hint: Option<&str>,
    ) -> Result<Vec<TransformGraphResource>, TransformGraphRequestError> {
        let resolved = self.resolved(reference);
        if !reference.contains('*') {
            return self.resource(&resolved).map(|resource| vec![resource]);
        }
        validate_transform_graph_import_glob(reference, self.config_uri)?;
        let matcher = transform_graph_reference_matcher(&resolved).map_err(|message| {
            TransformGraphRequestError::diagnostic(
                self.config_uri,
                "cem.transform_config.import_glob_unsupported",
                message,
            )
        })?;
        let mut matches = self
            .resources
            .keys()
            .filter(|uri| matcher.is_match(uri))
            .map(|uri| self.resource(uri))
            .collect::<Result<Vec<_>, _>>()?;
        matches.sort_by(|left, right| left.uri.cmp(&right.uri));
        validate_import_match_count(reference, self.config_uri, matches.len())?;
        Ok(matches)
    }

    fn read_resource(
        &self,
        reference: &str,
        _purpose: ResolvePurpose,
        _content_type_hint: Option<&str>,
    ) -> Result<TransformGraphResource, TransformGraphRequestError> {
        self.resource(&self.resolved(reference))
    }

    fn output_uri(&self, reference: &str) -> Result<String, TransformGraphRequestError> {
        Ok(self.resolved(reference))
    }

    fn binding_uri(&self, uri: &str) -> String {
        relative_transform_graph_reference(self.config_uri, uri)
    }
}

#[derive(Debug)]
pub struct FilesystemTransformGraphResourceProvider<'a> {
    context: &'a EngineContext,
    config_uri: String,
    config_local_path: Option<PathBuf>,
}

impl<'a> FilesystemTransformGraphResourceProvider<'a> {
    pub fn new(context: &'a EngineContext, config_uri: impl Into<String>) -> Self {
        let config_uri = config_uri.into();
        let config_local_path = local_path_or_file_uri(Path::new(&config_uri), "config path")
            .ok()
            .map(|path| path.into_owned());
        Self {
            context,
            config_uri,
            config_local_path,
        }
    }

    fn resolved(&self, reference: &str) -> String {
        if Path::new(reference).is_absolute()
            || uri_scheme(reference).is_some() && !is_windows_drive_path(reference)
        {
            return reference.to_owned();
        }
        if let Some(config_dir) = self.config_local_path.as_deref().and_then(Path::parent) {
            return config_dir.join(reference).display().to_string();
        }
        resolve_transform_graph_reference(&self.config_uri, reference)
    }

    fn read_resolved(
        &self,
        uri: &str,
        purpose: ResolvePurpose,
        content_type_hint: Option<&str>,
    ) -> Result<TransformGraphResource, TransformGraphRequestError> {
        self.context.ensure_active()?;
        let path = Path::new(uri);
        let raw = path.to_string_lossy();
        let (bytes, resolved_content_type) = if raw.starts_with("file://") {
            if let Some(local) = local_file_uri_to_path(&raw) {
                (read_local_bytes(self.context, &local)?, None)
            } else {
                self.read_registered(uri, purpose, content_type_hint)?
            }
        } else if uri_scheme(&raw).is_some() && !is_windows_drive_path(&raw) {
            self.read_registered(uri, purpose, content_type_hint)?
        } else {
            (read_local_bytes(self.context, path)?, None)
        };
        Ok(TransformGraphResource {
            uri: uri.to_owned(),
            bytes,
            identity: None,
            content_type: resolved_content_type,
        })
    }

    fn read_registered(
        &self,
        uri: &str,
        purpose: ResolvePurpose,
        content_type_hint: Option<&str>,
    ) -> Result<(Vec<u8>, Option<String>), TransformGraphRequestError> {
        let mut request = ResolveRequest::new(uri, purpose, ResolveDirection::Read);
        if let Some(content_type_hint) = content_type_hint {
            request = request.with_content_type_hint(content_type_hint);
        }
        match self.context.resolver_registry.read_with_control(
            &request,
            &self.context.operation_control,
            crate::operation_control::ROOT_EXECUTION_SCOPE_ID,
        ) {
            Ok(read) => Ok((read.bytes, read.content_type)),
            Err(ResolverDiagnostic::Cancelled { source_map, .. }) => {
                Err(EngineError::Cancelled { source_map }.into())
            }
            Err(error) => Err(EngineError::Io {
                path: PathBuf::from(uri),
                source: io::Error::other(error),
            }
            .into()),
        }
    }

    fn local_import_paths(&self, pattern: &str) -> Result<Vec<String>, TransformGraphRequestError> {
        validate_transform_graph_import_glob(pattern, &self.config_uri)?;
        let resolved = self.resolved(pattern);
        let pattern_path = Path::new(&resolved);
        let file = pattern_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (prefix, suffix) = file.split_once('*').unwrap_or(("", ""));
        let parent = transform_graph_glob_parent(pattern_path);
        self.context.ensure_active()?;
        let matches = collect_transform_graph_import_glob_matches(&parent, prefix, suffix)
            .map_err(|source| EngineError::Io {
                path: parent.clone(),
                source,
            })?;
        self.context.ensure_active()?;
        validate_import_match_count(pattern, &self.config_uri, matches.len())?;
        Ok(matches
            .into_iter()
            .map(|path| path.display().to_string())
            .collect())
    }

    fn resolver_import_uris(
        &self,
        pattern: &str,
    ) -> Result<Vec<String>, TransformGraphRequestError> {
        validate_transform_graph_import_glob(pattern, &self.config_uri)?;
        let resolved = self.resolved(pattern);
        let request = ResolveListRequest::new(&resolved, ResolvePurpose::Input)
            .with_max_entries(TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES + 1);
        let mut entries = match self.context.resolver_registry.list_with_control(
            &request,
            &self.context.operation_control,
            crate::operation_control::ROOT_EXECUTION_SCOPE_ID,
        ) {
            Ok(entries) => entries,
            Err(ResolverDiagnostic::UnsupportedResolver { .. }) => {
                return Err(TransformGraphRequestError::diagnostic(
                    &self.config_uri,
                    "cem.transform_config.import_glob_resolver_unsupported",
                    format!("import glob `{pattern}` requires a resolver with list support"),
                ));
            }
            Err(ResolverDiagnostic::Cancelled { source_map, .. }) => {
                return Err(EngineError::Cancelled { source_map }.into());
            }
            Err(error) => {
                return Err(TransformGraphRequestError::diagnostic(
                    &self.config_uri,
                    "cem.transform_config.import_glob_resolver_error",
                    format!("import glob `{pattern}` failed during resolver listing: {error}"),
                ));
            }
        };
        entries.sort_by(|left, right| left.uri.cmp(&right.uri));
        validate_import_match_count(pattern, &self.config_uri, entries.len())?;
        Ok(entries.into_iter().map(|entry| entry.uri).collect())
    }
}

impl TransformGraphResourceProvider for FilesystemTransformGraphResourceProvider<'_> {
    fn import_resources(
        &self,
        reference: &str,
        content_type_hint: Option<&str>,
    ) -> Result<Vec<TransformGraphResource>, TransformGraphRequestError> {
        let uris = if !reference.contains('*') {
            vec![self.resolved(reference)]
        } else {
            let resolved = self.resolved(reference);
            if uri_scheme(&resolved).is_some() && !is_windows_drive_path(&resolved) {
                self.resolver_import_uris(reference)?
            } else {
                self.local_import_paths(reference)?
            }
        };
        uris.into_iter()
            .map(|uri| self.read_resolved(&uri, ResolvePurpose::Input, content_type_hint))
            .collect()
    }

    fn read_resource(
        &self,
        reference: &str,
        purpose: ResolvePurpose,
        content_type_hint: Option<&str>,
    ) -> Result<TransformGraphResource, TransformGraphRequestError> {
        self.read_resolved(&self.resolved(reference), purpose, content_type_hint)
    }

    fn read_resource_relative_to(
        &self,
        base_uri: &str,
        reference: &str,
        purpose: ResolvePurpose,
        content_type_hint: Option<&str>,
    ) -> Result<TransformGraphResource, TransformGraphRequestError> {
        let resolved = resolve_transform_graph_reference(base_uri, reference);
        self.read_resolved(&resolved, purpose, content_type_hint)
    }

    fn output_uri(&self, reference: &str) -> Result<String, TransformGraphRequestError> {
        Ok(self.resolved(reference))
    }

    fn binding_uri(&self, uri: &str) -> String {
        if let Some(config_dir) = self.config_local_path.as_deref().and_then(Path::parent) {
            if let Ok(relative) = Path::new(uri).strip_prefix(config_dir) {
                return path_display_slash(relative);
            }
        }
        relative_transform_graph_reference(&self.config_uri, uri)
    }
}

fn read_local_bytes(
    context: &EngineContext,
    path: &Path,
) -> Result<Vec<u8>, TransformGraphRequestError> {
    context.ensure_active()?;
    let bytes = fs::read(path).map_err(|source| EngineError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    context.ensure_active()?;
    Ok(bytes)
}

pub fn resolve_transform_graph_reference(config_uri: &str, reference: &str) -> String {
    if uri_scheme(reference).is_some()
        || Path::new(reference).is_absolute()
        || is_windows_drive_path(reference)
    {
        return reference.to_owned();
    }
    let base = config_uri
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or(config_uri);
    normalize_transform_graph_uri(&format!("{base}/{reference}"))
}

fn normalize_transform_graph_uri(uri: &str) -> String {
    let (prefix, path) = if let Some(scheme) = uri.find("://") {
        let authority_start = scheme + 3;
        match uri[authority_start..].find('/') {
            Some(path_start) => {
                let path_start = authority_start + path_start;
                (&uri[..path_start], &uri[path_start + 1..])
            }
            None => (uri, ""),
        }
    } else if let Some(path) = uri.strip_prefix('/') {
        ("/", path)
    } else {
        ("", uri)
    };
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }
    match prefix {
        "" => segments.join("/"),
        "/" => format!("/{}", segments.join("/")),
        _ => format!("{prefix}/{}", segments.join("/")),
    }
}

fn relative_transform_graph_reference(config_uri: &str, uri: &str) -> String {
    let base = config_uri
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or(config_uri);
    uri.strip_prefix(base)
        .map(|relative| relative.trim_start_matches('/').to_owned())
        .unwrap_or_else(|| uri.to_owned())
}

fn transform_graph_reference_matcher(pattern: &str) -> Result<Regex, String> {
    let mut expression = String::from("^");
    let mut characters = pattern.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '*' if characters.peek() == Some(&'*') => {
                characters.next();
                expression.push_str(".*");
            }
            '*' => expression.push_str("[^/]*"),
            '?' => expression.push_str("[^/]"),
            '{' => {
                let mut name = String::new();
                loop {
                    match characters.next() {
                        Some('}') => break,
                        Some(character) => name.push(character),
                        None => {
                            return Err(format!(
                                "reference pattern `{pattern}` has an unclosed binding"
                            ));
                        }
                    }
                }
                if name.trim().is_empty() {
                    return Err(format!(
                        "reference pattern `{pattern}` has an empty binding"
                    ));
                }
                expression.push_str("[^/]+");
            }
            '}' => {
                return Err(format!(
                    "reference pattern `{pattern}` has an unmatched closing binding"
                ));
            }
            '[' | ']' => {
                return Err(format!(
                    "reference pattern `{pattern}` uses unsupported character-class syntax"
                ));
            }
            _ => expression.push_str(&regex::escape(&character.to_string())),
        }
    }
    expression.push('$');
    Regex::new(&expression).map_err(|error| error.to_string())
}

/// Match one resolved resource URI against the exact graph wildcard and
/// binding grammar used by both filesystem and manifest-backed lowering.
/// Host adapters use this during pre-request discovery so they never need a
/// second implementation of graph selector semantics.
pub fn transform_graph_reference_matches(
    pattern: &str,
    candidate: &str,
) -> Result<bool, String> {
    transform_graph_reference_matcher(pattern).map(|matcher| matcher.is_match(candidate))
}

fn validate_transform_graph_import_glob(
    raw: &str,
    config_uri: &str,
) -> Result<(), TransformGraphRequestError> {
    let (dir, file) = raw.rsplit_once('/').unwrap_or(("", raw));
    if file.matches('*').count() != 1 {
        return Err(TransformGraphRequestError::diagnostic(
            config_uri,
            "cem.transform_config.import_glob_unsupported",
            format!("import glob `{raw}` must contain exactly one `*` in the file name"),
        ));
    }
    let mut recursive_segments = 0;
    for segment in dir.split('/') {
        if segment == "**" {
            recursive_segments += 1;
        } else if segment.contains('*') {
            return Err(TransformGraphRequestError::diagnostic(
                config_uri,
                "cem.transform_config.import_glob_unsupported",
                format!("import glob `{raw}` can only use `**` as a complete directory segment"),
            ));
        }
    }
    if recursive_segments > 1 {
        return Err(TransformGraphRequestError::diagnostic(
            config_uri,
            "cem.transform_config.import_glob_unsupported",
            format!("import glob `{raw}` can contain at most one `**` directory segment"),
        ));
    }
    Ok(())
}

fn validate_import_match_count(
    raw: &str,
    config_uri: &str,
    count: usize,
) -> Result<(), TransformGraphRequestError> {
    if count == 0 {
        return Err(TransformGraphRequestError::diagnostic(
            config_uri,
            "cem.transform_config.import_glob_empty",
            format!("import glob `{raw}` matched no files"),
        ));
    }
    if count > TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES {
        return Err(TransformGraphRequestError::diagnostic(
            config_uri,
            "cem.transform_config.import_glob_too_many",
            format!(
                "import glob `{raw}` matched more than {TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES} files"
            ),
        ));
    }
    Ok(())
}

fn path_display_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn transform_graph_glob_parent(pattern_path: &Path) -> PathBuf {
    pattern_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn split_recursive_glob_parent(parent: &Path) -> Option<(PathBuf, PathBuf)> {
    let recursive = OsStr::new("**");
    let mut root = PathBuf::new();
    let mut suffix = PathBuf::new();
    let mut seen_recursive = false;
    for component in parent.components() {
        if component.as_os_str() == recursive {
            seen_recursive = true;
        } else if seen_recursive {
            suffix.push(component.as_os_str());
        } else {
            root.push(component.as_os_str());
        }
    }
    seen_recursive.then(|| {
        if root.as_os_str().is_empty() {
            root.push(".");
        }
        (root, suffix)
    })
}

fn collect_one_level_glob_matches(
    parent: &Path,
    prefix: &str,
    suffix: &str,
) -> io::Result<Vec<PathBuf>> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let file = entry.file_name().to_string_lossy().into_owned();
            if file.starts_with(prefix) && file.ends_with(suffix) {
                matches.push(entry.path());
            }
        }
    }
    Ok(matches)
}

fn collect_recursive_glob_matches(
    current: &Path,
    suffix_dir: &Path,
    prefix: &str,
    suffix: &str,
    matches: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let candidate = if suffix_dir.as_os_str().is_empty() {
        current.to_path_buf()
    } else {
        current.join(suffix_dir)
    };
    if candidate.is_dir() {
        matches.extend(collect_one_level_glob_matches(&candidate, prefix, suffix)?);
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_recursive_glob_matches(&entry.path(), suffix_dir, prefix, suffix, matches)?;
        }
    }
    Ok(())
}

fn collect_transform_graph_import_glob_matches(
    parent: &Path,
    prefix: &str,
    suffix: &str,
) -> io::Result<Vec<PathBuf>> {
    let mut matches = if let Some((root, suffix_dir)) = split_recursive_glob_parent(parent) {
        let mut matches = Vec::new();
        collect_recursive_glob_matches(&root, &suffix_dir, prefix, suffix, &mut matches)?;
        matches
    } else {
        collect_one_level_glob_matches(parent, prefix, suffix)?
    };
    matches.sort();
    Ok(matches)
}

#[derive(Debug, Clone)]
struct ArtifactVariant {
    id: String,
    bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct ArtifactOutputRoute {
    destination: String,
    target: Option<FormatIdentity>,
}

type JoinGroup = (
    String,
    Vec<(String, ArtifactVariant)>,
    BTreeMap<String, String>,
);

pub fn lower_transform_graph_request(
    context: &EngineContext,
    graph: &TransformGraphConfig,
    provider: &dyn TransformGraphResourceProvider,
    config_uri: &str,
    preserve_source_offsets: bool,
) -> Result<TransformGraphRequest, TransformGraphRequestError> {
    let mut lowerer = TransformGraphRequestLowerer {
        context,
        graph,
        provider,
        config_uri,
        imports: Vec::new(),
        joins: Vec::new(),
        conversions: Vec::new(),
        stages: Vec::new(),
        importmap_rewrites: Vec::new(),
        exports: Vec::new(),
        edges: Vec::new(),
        variants: BTreeMap::new(),
        output_routes: BTreeMap::new(),
        module_assets: BTreeMap::new(),
        module_asset_records: Vec::new(),
        next_scope_id: 0,
    };
    lowerer.lower()?;
    apply_stage_targets(&mut lowerer.stages, &lowerer.exports);
    Ok(TransformGraphRequest {
        imports: lowerer.imports,
        joins: lowerer.joins,
        conversions: lowerer.conversions,
        stages: lowerer.stages,
        importmap_rewrites: lowerer.importmap_rewrites,
        exports: lowerer.exports,
        edges: lowerer.edges,
        module_asset_manifest: module_asset_manifest(lowerer.module_asset_records),
        preserve_source_offsets,
        context: context.clone(),
        execution_policy: TransformExecutionPolicy::default(),
    })
}

struct TransformGraphRequestLowerer<'a> {
    context: &'a EngineContext,
    graph: &'a TransformGraphConfig,
    provider: &'a dyn TransformGraphResourceProvider,
    config_uri: &'a str,
    imports: Vec<TransformGraphImport>,
    joins: Vec<TransformGraphJoin>,
    conversions: Vec<TransformGraphConversion>,
    stages: Vec<TransformGraphStage>,
    importmap_rewrites: Vec<TransformGraphImportMapRewrite>,
    exports: Vec<TransformGraphExport>,
    edges: Vec<TransformGraphDependency>,
    variants: BTreeMap<String, Vec<ArtifactVariant>>,
    output_routes: BTreeMap<String, ArtifactOutputRoute>,
    module_assets: BTreeMap<String, Vec<PendingModuleAsset>>,
    module_asset_records: Vec<TransformGraphModuleAsset>,
    next_scope_id: u32,
}

#[derive(Debug, Clone)]
struct PendingModuleAsset {
    import_id: String,
    specifier: String,
    source_map_uri: String,
    source_uri: String,
    target: String,
    content_type: String,
    source_byte_length: u64,
    source_sha256: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Debug)]
struct ModuleMapDocument {
    uri: String,
    schema: Option<String>,
    imports: BTreeMap<String, ModuleMapImport>,
    resources: BTreeMap<String, ModuleMapResource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModuleMapImport {
    path: String,
    content_type: String,
    module_imports: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModuleMapResource {
    path: String,
    content_type: String,
    module_imports: BTreeMap<String, String>,
}

impl TransformGraphRequestLowerer<'_> {
    fn lower(&mut self) -> Result<(), TransformGraphRequestError> {
        for node in &self.graph.nodes {
            match node.kind {
                TransformGraphNodeKind::Import => self.lower_import(node)?,
                TransformGraphNodeKind::Join => self.lower_join(node)?,
                TransformGraphNodeKind::Convert => self.lower_convert(node)?,
                TransformGraphNodeKind::Transform => self.lower_transform(node)?,
                TransformGraphNodeKind::ImportMapRewrite => self.lower_importmap(node)?,
                TransformGraphNodeKind::Export => self.lower_export(node)?,
            }
        }
        Ok(())
    }

    fn lower_import(
        &mut self,
        node: &TransformGraphNode,
    ) -> Result<(), TransformGraphRequestError> {
        let src = required_field(self.config_uri, node, node.src.as_deref(), "src", "import")?;
        let resources = self
            .provider
            .import_resources(src, node.content_type.as_deref())?;
        let count = resources.len();
        let mut variants = Vec::with_capacity(count);
        for (index, resource) in resources.into_iter().enumerate() {
            let id = variant_id(&node.id, index, count);
            let bindings = resource_bindings(
                &resource.uri,
                &self.provider.binding_uri(&resource.uri),
                index,
            );
            let input = engine_input_from_resource(
                resource,
                node.content_type.clone(),
                node.schema.clone(),
            );
            let scheduler_scope_id = self.take_scope();
            self.imports.push(TransformGraphImport {
                id: id.clone(),
                input,
                opaque: node.opaque,
                scheduler_scope_id,
            });
            variants.push(ArtifactVariant { id, bindings });
        }
        self.variants.insert(node.id.clone(), variants);
        Ok(())
    }

    fn lower_join(&mut self, node: &TransformGraphNode) -> Result<(), TransformGraphRequestError> {
        let mode = node.join_mode.ok_or_else(|| {
            config_error(
                self.config_uri,
                "cem.transform_config.join_mode_missing",
                format!("join node `{}` requires @mode", node.id),
            )
        })?;
        let (input_ref, input_role) = primary_ref(self.graph, node, self.config_uri)?;
        let input_variants = variants_for_ref(
            &self.variants,
            &node.id,
            "input",
            &input_ref,
            self.config_uri,
        )?;
        let groups = match mode {
            ConfigJoinMode::Collect => {
                let count = input_variants.len();
                vec![(
                    node.id.clone(),
                    input_variants
                        .into_iter()
                        .map(|variant| ("primary".to_owned(), variant))
                        .collect(),
                    BTreeMap::from([("count".to_owned(), count.to_string())]),
                )]
            }
            ConfigJoinMode::GroupBy => group_by_join_groups(node, input_variants, self.config_uri)?,
            ConfigJoinMode::MatchBy => {
                match_by_join_groups(node, input_variants, &self.variants, self.config_uri)?
            }
            ConfigJoinMode::Zip => {
                zip_join_groups(node, input_variants, &self.variants, self.config_uri)?
            }
        };
        let mut output_variants = Vec::new();
        for (join_id, input_variants, bindings) in groups {
            let input_names = if matches!(mode, ConfigJoinMode::MatchBy | ConfigJoinMode::Zip) {
                std::iter::once("primary".to_owned())
                    .chain(node.with.keys().cloned())
                    .collect()
            } else {
                input_variants
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            };
            let inputs = input_variants
                .iter()
                .map(|(name, variant)| {
                    let route = self.output_routes.get(&variant.id);
                    TransformGraphJoinInput {
                        input_name: name.clone(),
                        artifact_id: variant.id.clone(),
                        bindings: variant.bindings.clone(),
                        destination: route.map(|route| route.destination.clone()),
                        target: route.and_then(|route| route.target.clone()),
                    }
                })
                .collect();
            let scheduler_scope_id = self.take_scope();
            self.joins.push(TransformGraphJoin {
                id: join_id.clone(),
                mode: engine_join_mode(mode),
                input_names,
                inputs,
                bindings: bindings.clone(),
                scheduler_scope_id,
            });
            for (name, variant) in input_variants {
                self.edges.push(TransformGraphDependency {
                    from: variant.id,
                    to: join_id.clone(),
                    role: if name == "primary" {
                        engine_dependency_role(input_role)
                    } else {
                        TransformGraphDependencyRole::SecondaryInput
                    },
                });
            }
            output_variants.push(ArtifactVariant {
                id: join_id,
                bindings,
            });
        }
        self.variants.insert(node.id.clone(), output_variants);
        Ok(())
    }

    fn lower_transform(
        &mut self,
        node: &TransformGraphNode,
    ) -> Result<(), TransformGraphRequestError> {
        let src = required_field(
            self.config_uri,
            node,
            node.src.as_deref(),
            "src",
            "transform",
        )?;
        let (primary_ref, primary_role) = primary_ref(self.graph, node, self.config_uri)?;
        let primary_variants = variants_for_ref(
            &self.variants,
            &node.id,
            "input",
            &primary_ref,
            self.config_uri,
        )?;
        let count = primary_variants.len();
        let mut stage_variants = Vec::with_capacity(count);
        for (index, primary_variant) in primary_variants.into_iter().enumerate() {
            let stage_id = variant_id(&node.id, index, count);
            let expanded_src = expand_binding_template(
                src,
                &primary_variant.bindings,
                self.config_uri,
                "template path template",
                "template_binding",
            )?;
            let resource = self.provider.read_resource(
                &expanded_src,
                ResolvePurpose::Template,
                node.template_content_type.as_deref(),
            )?;
            let template = template_input_from_resource(
                resource,
                node.template_content_type.clone(),
                node.template_schema.clone(),
            );
            let identity = template
                .identity
                .clone()
                .unwrap_or_else(|| template.root_scope.format_identity());
            let template_kind = match node.template_kind {
                Some(kind) => kind,
                None => engine::classify_transform_template_identity_with_registry(
                    &identity,
                    &self.context.template_adapter_registry,
                )
                .map_err(|error| {
                    config_error(
                        self.config_uri,
                        engine::TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
                        error.to_string(),
                    )
                })?,
            };
            let template_entrypoint = template_entrypoint(node.entrypoint.as_deref());
            let params = stage_params(node, &primary_variant.bindings, self.config_uri)?;
            validate_template_surface(
                template_kind,
                &template_entrypoint,
                &params,
                self.config_uri,
            )?;
            let mut secondary_inputs = BTreeMap::new();
            for (name, target) in &node.with {
                let secondary = single_variant_for_ref(
                    &self.variants,
                    &node.id,
                    &format!("with:{name}"),
                    target,
                    self.config_uri,
                )?;
                secondary_inputs.insert(name.clone(), secondary.id.clone());
                self.edges.push(TransformGraphDependency {
                    from: secondary.id,
                    to: stage_id.clone(),
                    role: TransformGraphDependencyRole::SecondaryInput,
                });
            }
            let template_load = self.take_scope();
            let execution = self.take_scope();
            self.stages.push(TransformGraphStage {
                id: stage_id.clone(),
                template,
                template_kind,
                template_entrypoint: template_entrypoint.clone(),
                params: params.clone(),
                execution_policy: transform_execution_policy(
                    template_kind,
                    &template_entrypoint,
                    &params,
                ),
                target: None,
                primary_input: primary_variant.id.clone(),
                secondary_inputs,
                scheduler_scope_ids: TransformStageSchedulerScopeIds {
                    template_load,
                    execution,
                },
            });
            self.edges.push(TransformGraphDependency {
                from: primary_variant.id,
                to: stage_id.clone(),
                role: engine_dependency_role(primary_role),
            });
            stage_variants.push(ArtifactVariant {
                id: stage_id,
                bindings: primary_variant.bindings,
            });
        }
        self.variants.insert(node.id.clone(), stage_variants);
        Ok(())
    }

    fn lower_convert(
        &mut self,
        node: &TransformGraphNode,
    ) -> Result<(), TransformGraphRequestError> {
        let (primary_ref, primary_role) = primary_ref(self.graph, node, self.config_uri)?;
        let primary_variants = variants_for_ref(
            &self.variants,
            &node.id,
            "input",
            &primary_ref,
            self.config_uri,
        )?;
        let count = primary_variants.len();
        let mut converted_variants = Vec::with_capacity(count);
        for (index, primary_variant) in primary_variants.into_iter().enumerate() {
            let conversion_id = variant_id(&node.id, index, count);
            let target_scope = resource_scope(
                self.config_uri,
                None,
                node.content_type.clone(),
                node.schema.clone(),
            );
            let target = target_scope.format_identity_option().ok_or_else(|| {
                config_error(
                    self.config_uri,
                    "cem.transform_config.convert_target_missing",
                    format!(
                        "convert node `{}` requires target @content-type or @schema",
                        node.id
                    ),
                )
            })?;
            let scheduler_scope_id = self.take_scope();
            self.conversions.push(TransformGraphConversion {
                id: conversion_id.clone(),
                primary_input: primary_variant.id.clone(),
                converter_id: node.converter.clone(),
                target,
                target_scope,
                scheduler_scope_id,
            });
            self.edges.push(TransformGraphDependency {
                from: primary_variant.id,
                to: conversion_id.clone(),
                role: engine_dependency_role(primary_role),
            });
            converted_variants.push(ArtifactVariant {
                id: conversion_id,
                bindings: primary_variant.bindings,
            });
        }
        self.variants.insert(node.id.clone(), converted_variants);
        Ok(())
    }

    fn lower_importmap(
        &mut self,
        node: &TransformGraphNode,
    ) -> Result<(), TransformGraphRequestError> {
        let target_map = required_field(
            self.config_uri,
            node,
            node.target_map.as_deref(),
            "target-map",
            "rewrite-importmap",
        )?;
        let (primary_ref, primary_role) = primary_ref(self.graph, node, self.config_uri)?;
        let primary_variants = variants_for_ref(
            &self.variants,
            &node.id,
            "input",
            &primary_ref,
            self.config_uri,
        )?;
        let count = primary_variants.len();
        let mut rewrite_variants = Vec::with_capacity(count);
        for (index, primary_variant) in primary_variants.into_iter().enumerate() {
            let rewrite_id = variant_id(&node.id, index, count);
            let source_map = if let Some(source_map) = node.source_map.as_deref() {
                Some(self.importmap_document(source_map, &primary_variant.bindings, false)?)
            } else {
                None
            };
            let target_map =
                self.importmap_document(target_map, &primary_variant.bindings, true)?;
            let source_module_map_schema = source_map
                .as_ref()
                .and_then(|document| document.schema.as_deref())
                .filter(|schema| is_module_map_schema(schema));
            let target_module_map_schema = target_map
                .schema
                .as_deref()
                .filter(|schema| is_module_map_schema(schema));
            match (source_module_map_schema, target_module_map_schema) {
                (Some(source_schema), Some(target_schema)) if source_schema == target_schema => {
                    self.lower_module_assets(
                        &rewrite_id,
                        source_map.as_ref().expect("source module map"),
                        &target_map,
                    )?
                }
                (Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) => {
                    return Err(config_error(
                        self.config_uri,
                        "cem.transform_config.module_map_identity_mismatch",
                        format!(
                            "rewrite-importmap node `{}` must pair source and target documents with the same supported module-map schema",
                            node.id
                        ),
                    ));
                }
                (None, None) => {}
            }
            let source_imports = source_map
                .map(|document| {
                    document
                        .imports
                        .into_iter()
                        .map(|(specifier, entry)| (specifier, entry.path))
                        .collect()
                })
                .unwrap_or_default();
            let target_imports = target_map
                .imports
                .into_iter()
                .map(|(specifier, entry)| (specifier, entry.path))
                .collect();
            let scheduler_scope_id = self.take_scope();
            self.importmap_rewrites
                .push(TransformGraphImportMapRewrite {
                    id: rewrite_id.clone(),
                    primary_input: primary_variant.id.clone(),
                    source_imports,
                    target_imports,
                    mode: engine_importmap_rewrite_mode(node.rewrite_mode.unwrap_or(
                        transform_config::TransformGraphImportMapRewriteMode::ReplaceImports,
                    )),
                    missing_policy: engine_importmap_missing_policy(
                        node.missing_policy.unwrap_or(
                            transform_config::TransformGraphImportMapMissingPolicy::Error,
                        ),
                    ),
                    scheduler_scope_id,
                });
            self.edges.push(TransformGraphDependency {
                from: primary_variant.id,
                to: rewrite_id.clone(),
                role: engine_dependency_role(primary_role),
            });
            rewrite_variants.push(ArtifactVariant {
                id: rewrite_id,
                bindings: primary_variant.bindings,
            });
        }
        self.variants.insert(node.id.clone(), rewrite_variants);
        Ok(())
    }

    fn importmap_document(
        &self,
        raw: &str,
        bindings: &BTreeMap<String, String>,
        required: bool,
    ) -> Result<ModuleMapDocument, TransformGraphRequestError> {
        let expanded = expand_binding_template(
            raw,
            bindings,
            self.config_uri,
            "importmap path template",
            "param_binding",
        )?;
        let resource = self.provider.read_resource(
            &expanded,
            ResolvePurpose::ModuleMap,
            Some(MODULE_MAP_CONTENT_TYPE),
        )?;
        let value: serde_json::Value =
            serde_json::from_slice(&resource.bytes).map_err(|error| {
                config_error(
                    self.config_uri,
                    "cem.transform_config.importmap_json_invalid",
                    format!("importmap `{}` is not valid JSON: {error}", resource.uri),
                )
            })?;
        let schema = match value.get("$schema") {
            Some(serde_json::Value::String(schema)) if is_module_map_schema(schema) => {
                Some(schema.clone())
            }
            Some(serde_json::Value::String(schema)) => {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.schema_unsupported",
                    format!(
                        "module map `{}` uses unsupported `$schema` `{schema}`; expected `{MODULE_MAP_SCHEMA_URI}`, `{MODULE_MAP_V2_SCHEMA_URI}`, or `{MODULE_MAP_V3_SCHEMA_URI}`",
                        resource.uri
                    ),
                ));
            }
            Some(_) => {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.schema_invalid",
                    format!("module map `{}` `$schema` must be a string", resource.uri),
                ));
            }
            None => None,
        };
        let Some(imports) = value.get("imports").and_then(serde_json::Value::as_object) else {
            if required {
                return Err(config_error(
                    self.config_uri,
                    "cem.transform_config.importmap_imports_missing",
                    format!(
                        "importmap `{}` requires an object `imports` map",
                        resource.uri
                    ),
                ));
            }
            return Ok(ModuleMapDocument {
                uri: resource.uri,
                schema,
                imports: BTreeMap::new(),
                resources: BTreeMap::new(),
            });
        };
        let imports = imports
            .iter()
            .map(|(key, value)| {
                if schema.as_deref() == Some(MODULE_MAP_V3_SCHEMA_URI) {
                    parse_module_map_v3_asset_entry(
                        self.config_uri,
                        &resource.uri,
                        "import",
                        key,
                        value,
                    )
                    .map(|entry| {
                        (
                            key.clone(),
                            ModuleMapImport {
                                path: entry.path,
                                content_type: entry.content_type,
                                module_imports: entry.module_imports,
                            },
                        )
                    })
                } else {
                    value
                        .as_str()
                        .map(|value| {
                            (
                                key.clone(),
                                ModuleMapImport {
                                    path: value.to_owned(),
                                    content_type: "text/javascript".to_owned(),
                                    module_imports: BTreeMap::new(),
                                },
                            )
                        })
                        .ok_or_else(|| {
                            config_error(
                                self.config_uri,
                                "cem.transform_config.importmap_entry_invalid",
                                format!(
                                    "importmap `{}` entry `{key}` must be a string",
                                    resource.uri
                                ),
                            )
                        })
                }
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let resources = match value.get("resources") {
            None => BTreeMap::new(),
            Some(_)
                if !matches!(
                    schema.as_deref(),
                    Some(MODULE_MAP_V2_SCHEMA_URI | MODULE_MAP_V3_SCHEMA_URI)
                ) =>
            {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.resources_unsupported",
                    format!(
                        "module map `{}` may declare `resources` only with schema `{MODULE_MAP_V2_SCHEMA_URI}` or `{MODULE_MAP_V3_SCHEMA_URI}`",
                        resource.uri
                    ),
                ));
            }
            Some(serde_json::Value::Object(resources)) => resources
                .iter()
                .map(|(name, value)| {
                    if schema.as_deref() == Some(MODULE_MAP_V3_SCHEMA_URI) {
                        return parse_module_map_v3_asset_entry(
                            self.config_uri,
                            &resource.uri,
                            "resource",
                            name,
                            value,
                        )
                        .map(|entry| {
                            (
                                name.clone(),
                                ModuleMapResource {
                                    path: entry.path,
                                    content_type: entry.content_type,
                                    module_imports: entry.module_imports,
                                },
                            )
                        });
                    }
                    let entry = value.as_object().ok_or_else(|| {
                        config_error(
                            self.config_uri,
                            "cem.module_map.resource_entry_invalid",
                            format!(
                                "module map `{}` resource `{name}` must be an object",
                                resource.uri
                            ),
                        )
                    })?;
                    let path = entry
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            config_error(
                                self.config_uri,
                                "cem.module_map.resource_entry_invalid",
                                format!(
                                    "module map `{}` resource `{name}` requires a non-empty string `path`",
                                    resource.uri
                                ),
                            )
                        })?;
                    let content_type = entry
                        .get("contentType")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            config_error(
                                self.config_uri,
                                "cem.module_map.resource_entry_invalid",
                                format!(
                                    "module map `{}` resource `{name}` requires a non-empty string `contentType`",
                                    resource.uri
                                ),
                            )
                        })?;
                    if entry
                        .keys()
                        .any(|key| !matches!(key.as_str(), "path" | "contentType"))
                    {
                        return Err(config_error(
                            self.config_uri,
                            "cem.module_map.resource_entry_invalid",
                            format!(
                                "module map `{}` resource `{name}` permits only `path` and `contentType`",
                                resource.uri
                            ),
                        ));
                    }
                    Ok((
                        name.clone(),
                        ModuleMapResource {
                            path: path.to_owned(),
                            content_type: content_type.to_owned(),
                            module_imports: BTreeMap::new(),
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?,
            Some(_) => {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.resources_invalid",
                    format!(
                        "module map `{}` `resources` must be an object",
                        resource.uri
                    ),
                ));
            }
        };
        Ok(ModuleMapDocument {
            uri: resource.uri,
            schema,
            imports,
            resources,
        })
    }

    fn lower_module_assets(
        &mut self,
        rewrite_id: &str,
        source_map: &ModuleMapDocument,
        target_map: &ModuleMapDocument,
    ) -> Result<(), TransformGraphRequestError> {
        let source_keys = source_map.imports.keys().collect::<BTreeSet<_>>();
        let target_keys = target_map.imports.keys().collect::<BTreeSet<_>>();
        if source_keys != target_keys {
            let missing = source_keys
                .difference(&target_keys)
                .map(|key| key.as_str())
                .collect::<Vec<_>>();
            let undeclared = target_keys
                .difference(&source_keys)
                .map(|key| key.as_str())
                .collect::<Vec<_>>();
            return Err(config_error(
                self.config_uri,
                "cem.module_map.entries_mismatch",
                format!(
                    "module-map source and target `imports` keys must match; missing targets: [{}]; undeclared targets: [{}]",
                    missing.join(", "),
                    undeclared.join(", ")
                ),
            ));
        }

        let source_resource_keys = source_map.resources.keys().collect::<BTreeSet<_>>();
        let target_resource_keys = target_map.resources.keys().collect::<BTreeSet<_>>();
        if source_resource_keys != target_resource_keys {
            let missing = source_resource_keys
                .difference(&target_resource_keys)
                .map(|key| key.as_str())
                .collect::<Vec<_>>();
            let undeclared = target_resource_keys
                .difference(&source_resource_keys)
                .map(|key| key.as_str())
                .collect::<Vec<_>>();
            return Err(config_error(
                self.config_uri,
                "cem.module_map.resource_entries_mismatch",
                format!(
                    "module-map source and target `resources` keys must match; missing targets: [{}]; undeclared targets: [{}]",
                    missing.join(", "),
                    undeclared.join(", ")
                ),
            ));
        }
        if let Some(name) = source_keys.intersection(&source_resource_keys).next() {
            return Err(config_error(
                self.config_uri,
                "cem.module_map.asset_identity_duplicate",
                format!(
                    "module-map asset identity `{name}` cannot be declared in both `imports` and `resources`"
                ),
            ));
        }

        let v3 = source_map.schema.as_deref() == Some(MODULE_MAP_V3_SCHEMA_URI);
        let mut deployment_assets = BTreeMap::new();
        for (identity, asset) in &target_map.imports {
            deployment_assets.insert(
                identity.clone(),
                (asset.path.clone(), content_type_essence(&asset.content_type)),
            );
        }
        for (identity, asset) in &target_map.resources {
            deployment_assets.insert(
                identity.clone(),
                (asset.path.clone(), content_type_essence(&asset.content_type)),
            );
        }

        let mut assets = Vec::with_capacity(source_map.imports.len() + source_map.resources.len());
        for (index, (specifier, source)) in source_map.imports.iter().enumerate() {
            validate_module_specifier(self.config_uri, specifier)?;
            let target = &target_map.imports[specifier];
            if source.module_imports != target.module_imports {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.module_imports_mismatch",
                    format!(
                        "module-map import `{specifier}` source and target `moduleImports` must match exactly"
                    ),
                ));
            }
            let source_content_type = validate_module_import_content_type(
                self.config_uri,
                specifier,
                &source.path,
                &source.content_type,
                v3,
            )?;
            let target_content_type = validate_module_import_content_type(
                self.config_uri,
                specifier,
                &target.path,
                &target.content_type,
                v3,
            )?;
            if source_content_type != target_content_type {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.import_type_mismatch",
                    format!(
                        "module-map import `{specifier}` source content type `{}` does not match target content type `{}`",
                        source.content_type, target.content_type
                    ),
                ));
            }
            validate_module_import_target(
                self.config_uri,
                specifier,
                &target.path,
                &target_content_type,
            )?;
            let mut resource = self.provider.read_resource_relative_to(
                &source_map.uri,
                &source.path,
                ResolvePurpose::Input,
                Some(&source_content_type),
            )?;
            let source_byte_length = resource.bytes.len() as u64;
            let source_sha256 = crate::command_service::sha256_hex(&resource.bytes);
            if v3 && source_content_type == "text/javascript" {
                resource.bytes = rewrite_declared_module_specifiers(
                    self.config_uri,
                    specifier,
                    &target.path,
                    &resource.bytes,
                    &source.module_imports,
                    &deployment_assets,
                )?;
            }
            let byte_length = resource.bytes.len() as u64;
            let sha256 = crate::command_service::sha256_hex(&resource.bytes);
            let source_uri = resource.uri.clone();
            let import_id = format!("{rewrite_id}:module:{index}");
            let scheduler_scope_id = self.take_scope();
            self.imports.push(TransformGraphImport {
                id: import_id.clone(),
                input: engine_input_from_resource(resource, Some(source_content_type.clone()), None),
                opaque: true,
                scheduler_scope_id,
            });
            assets.push(PendingModuleAsset {
                import_id,
                specifier: specifier.clone(),
                source_map_uri: source_map.uri.clone(),
                source_uri,
                target: target.path.clone(),
                content_type: source_content_type,
                source_byte_length,
                source_sha256,
                byte_length,
                sha256,
            });
        }
        for (index, (name, source)) in source_map.resources.iter().enumerate() {
            validate_module_specifier(self.config_uri, name)?;
            let target = &target_map.resources[name];
            if source.module_imports != target.module_imports {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.module_imports_mismatch",
                    format!(
                        "module-map resource `{name}` source and target `moduleImports` must match exactly"
                    ),
                ));
            }
            let source_content_type = validate_module_resource_content_type(
                self.config_uri,
                name,
                &source.path,
                &source.content_type,
            )?;
            let target_content_type = validate_module_resource_content_type(
                self.config_uri,
                name,
                &target.path,
                &target.content_type,
            )?;
            if source_content_type != target_content_type {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.resource_type_mismatch",
                    format!(
                        "module-map resource `{name}` source content type `{}` does not match target content type `{}`",
                        source.content_type, target.content_type
                    ),
                ));
            }
            validate_module_resource_target(
                self.config_uri,
                name,
                &target.path,
                &target_content_type,
            )?;
            if !v3 && !source.module_imports.is_empty() {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.module_imports_invalid",
                    format!("module-map resource `{name}` cannot declare `moduleImports` before v3"),
                ));
            }
            if !source.module_imports.is_empty() && source_content_type != "text/javascript" {
                return Err(config_error(
                    self.config_uri,
                    "cem.module_map.module_import_target_invalid",
                    format!(
                        "module-map resource `{name}` may declare `moduleImports` only for JavaScript"
                    ),
                ));
            }
            let mut resource = self.provider.read_resource_relative_to(
                &source_map.uri,
                &source.path,
                ResolvePurpose::Input,
                Some(&source_content_type),
            )?;
            let source_byte_length = resource.bytes.len() as u64;
            let source_sha256 = crate::command_service::sha256_hex(&resource.bytes);
            if v3 && source_content_type == "text/javascript" {
                resource.bytes = rewrite_declared_module_specifiers(
                    self.config_uri,
                    name,
                    &target.path,
                    &resource.bytes,
                    &source.module_imports,
                    &deployment_assets,
                )?;
            }
            let byte_length = resource.bytes.len() as u64;
            let sha256 = crate::command_service::sha256_hex(&resource.bytes);
            let source_uri = resource.uri.clone();
            let import_id = format!("{rewrite_id}:resource:{index}");
            let scheduler_scope_id = self.take_scope();
            self.imports.push(TransformGraphImport {
                id: import_id.clone(),
                input: engine_input_from_resource(
                    resource,
                    Some(source_content_type.clone()),
                    None,
                ),
                opaque: true,
                scheduler_scope_id,
            });
            assets.push(PendingModuleAsset {
                import_id,
                specifier: name.clone(),
                source_map_uri: source_map.uri.clone(),
                source_uri,
                target: target.path.clone(),
                content_type: source_content_type,
                source_byte_length,
                source_sha256,
                byte_length,
                sha256,
            });
        }
        self.module_assets.insert(rewrite_id.to_owned(), assets);
        Ok(())
    }

    fn lower_export(
        &mut self,
        node: &TransformGraphNode,
    ) -> Result<(), TransformGraphRequestError> {
        let out = required_field(self.config_uri, node, node.out.as_deref(), "out", "export")?;
        let (input_ref, input_role) = primary_ref(self.graph, node, self.config_uri)?;
        let input_variants = variants_for_ref(
            &self.variants,
            &node.id,
            "input",
            &input_ref,
            self.config_uri,
        )?;
        let count = input_variants.len();
        for (index, input_variant) in input_variants.into_iter().enumerate() {
            let export_id = variant_id(&node.id, index, count);
            let expanded = expand_binding_template(
                out,
                &input_variant.bindings,
                self.config_uri,
                "output path template",
                "output_binding",
            )?;
            let destination = self.provider.output_uri(&expanded)?;
            let target_scope = resource_scope(
                &destination,
                None,
                node.content_type.clone(),
                node.schema.clone(),
            );
            let target = target_scope.format_identity_option();
            let module_assets = self
                .module_assets
                .get(&input_variant.id)
                .cloned()
                .unwrap_or_default();
            if !module_assets.is_empty() && !module_asset_export_is_html(&target_scope) {
                return Err(config_error(
                    self.config_uri,
                    "cem.transform_config.module_map_html_export_required",
                    format!(
                        "module-map rewrite `{}` must export an HTML document before its JavaScript assets can be placed",
                        input_variant.id
                    ),
                ));
            }
            self.output_routes.insert(
                input_variant.id.clone(),
                ArtifactOutputRoute {
                    destination: destination.clone(),
                    target: target.clone(),
                },
            );
            let scheduler_scope_id = self.take_scope();
            self.exports.push(TransformGraphExport {
                id: export_id.clone(),
                input: input_variant.id.clone(),
                destination: Some(destination.clone()),
                target,
                target_scope,
                style_policy: node.style_policy.unwrap_or_default(),
                scheduler_scope_id,
            });
            self.edges.push(TransformGraphDependency {
                from: input_variant.id,
                to: export_id.clone(),
                role: engine_dependency_role(input_role),
            });
            for (asset_index, asset) in module_assets.into_iter().enumerate() {
                let asset_destination =
                    resolve_transform_graph_reference(&destination, &asset.target);
                let asset_scope = resource_scope(
                    &asset_destination,
                    None,
                    Some(asset.content_type.clone()),
                    None,
                );
                let asset_export_id = format!("{export_id}:module:{asset_index}");
                let scheduler_scope_id = self.take_scope();
                self.exports.push(TransformGraphExport {
                    id: asset_export_id.clone(),
                    input: asset.import_id.clone(),
                    destination: Some(asset_destination.clone()),
                    target: asset_scope.format_identity_option(),
                    target_scope: asset_scope,
                    style_policy: TransformGraphStylePolicy::Auto,
                    scheduler_scope_id,
                });
                self.edges.push(TransformGraphDependency {
                    from: asset.import_id,
                    to: asset_export_id,
                    role: TransformGraphDependencyRole::PrimaryInput,
                });
                self.module_asset_records.push(TransformGraphModuleAsset {
                    specifier: asset.specifier,
                    source_map_uri: asset.source_map_uri,
                    source_uri: asset.source_uri,
                    target: asset.target,
                    destination: asset_destination,
                    content_type: asset.content_type,
                    source_byte_length: asset.source_byte_length,
                    source_sha256: asset.source_sha256,
                    byte_length: asset.byte_length,
                    sha256: asset.sha256,
                });
            }
        }
        Ok(())
    }

    fn take_scope(&mut self) -> u32 {
        let scope = self.next_scope_id;
        self.next_scope_id += 1;
        scope
    }
}

fn engine_input_from_resource(
    resource: TransformGraphResource,
    content_type: Option<String>,
    schema: Option<String>,
) -> EngineInput {
    let root_scope = resource_scope(
        &resource.uri,
        resource.identity.as_ref(),
        content_type.or(resource.content_type),
        schema,
    );
    EngineInput {
        uri: resource.uri,
        bytes: resource.bytes,
        from_format: None,
        identity: root_scope.format_identity_option(),
        root_scope,
    }
}

fn template_input_from_resource(
    resource: TransformGraphResource,
    content_type: Option<String>,
    schema: Option<String>,
) -> TemplateInput {
    let root_scope = resource_scope(
        &resource.uri,
        resource.identity.as_ref(),
        content_type.or(resource.content_type),
        schema,
    );
    TemplateInput {
        uri: resource.uri,
        bytes: resource.bytes,
        identity: root_scope.format_identity_option(),
        root_scope,
    }
}

fn resource_scope(
    uri: &str,
    identity: Option<&FormatIdentity>,
    content_type: Option<String>,
    schema: Option<String>,
) -> ScopeConfig {
    ScopeConfig {
        default_content_type: content_type
            .or_else(|| identity.and_then(|identity| identity.content_type.clone()))
            .or_else(|| run_config::infer_content_type_from_path(uri)),
        schema: schema.or_else(|| identity.and_then(|identity| identity.schema.clone())),
        default_namespace: identity.and_then(|identity| identity.default_namespace.clone()),
        namespaces: identity
            .map(|identity| identity.namespaces.clone())
            .unwrap_or_default(),
        base_uri: identity.and_then(|identity| identity.base_uri.clone()),
        ..ScopeConfig::default()
    }
}

fn resource_bindings(uri: &str, binding_uri: &str, index: usize) -> BTreeMap<String, String> {
    let path = Path::new(binding_uri);
    let file = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path.parent().map(path_display_slash).unwrap_or_default();
    BTreeMap::from([
        ("src".to_owned(), uri.to_owned()),
        ("path".to_owned(), path_display_slash(path)),
        ("dir".to_owned(), dir),
        ("file".to_owned(), file),
        ("stem".to_owned(), stem),
        ("ext".to_owned(), ext),
        ("index".to_owned(), index.to_string()),
    ])
}

fn required_field<'a>(
    config_uri: &str,
    node: &TransformGraphNode,
    value: Option<&'a str>,
    field: &str,
    kind: &str,
) -> Result<&'a str, TransformGraphRequestError> {
    value.ok_or_else(|| {
        config_error(
            config_uri,
            "cem.transform_config.required_field_missing",
            format!("{kind} node `{}` requires @{field}", node.id),
        )
    })
}

fn config_error(uri: &str, code: &str, message: impl Into<String>) -> TransformGraphRequestError {
    TransformGraphRequestError::diagnostic(uri, code, message)
}

struct ParsedModuleMapV3AssetEntry {
    path: String,
    content_type: String,
    module_imports: BTreeMap<String, String>,
}

fn parse_module_map_v3_asset_entry(
    config_uri: &str,
    map_uri: &str,
    kind: &str,
    identity: &str,
    value: &serde_json::Value,
) -> Result<ParsedModuleMapV3AssetEntry, TransformGraphRequestError> {
    let diagnostic = if kind == "import" {
        "cem.module_map.import_entry_invalid"
    } else {
        "cem.module_map.resource_entry_invalid"
    };
    let entry = value.as_object().ok_or_else(|| {
        config_error(
            config_uri,
            diagnostic,
            format!("module map `{map_uri}` {kind} `{identity}` must be an object"),
        )
    })?;
    if entry
        .keys()
        .any(|key| !matches!(key.as_str(), "path" | "contentType" | "moduleImports"))
    {
        return Err(config_error(
            config_uri,
            diagnostic,
            format!(
                "module map `{map_uri}` {kind} `{identity}` permits only `path`, `contentType`, and `moduleImports`"
            ),
        ));
    }
    let path = entry
        .get("path")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            config_error(
                config_uri,
                diagnostic,
                format!(
                    "module map `{map_uri}` {kind} `{identity}` requires a non-empty string `path`"
                ),
            )
        })?;
    let content_type = entry
        .get("contentType")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            config_error(
                config_uri,
                diagnostic,
                format!(
                    "module map `{map_uri}` {kind} `{identity}` requires a non-empty string `contentType`"
                ),
            )
        })?;
    let module_imports = match entry.get("moduleImports") {
        None => BTreeMap::new(),
        Some(serde_json::Value::Object(edges)) => edges
            .iter()
            .map(|(specifier, asset)| {
                validate_module_specifier(config_uri, specifier)?;
                let asset = asset
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        config_error(
                            config_uri,
                            "cem.module_map.module_imports_invalid",
                            format!(
                                "module map `{map_uri}` {kind} `{identity}` module import `{specifier}` must name a declared asset"
                            ),
                        )
                    })?;
                validate_module_specifier(config_uri, asset)?;
                Ok::<_, TransformGraphRequestError>((specifier.clone(), asset.to_owned()))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?,
        Some(_) => {
            return Err(config_error(
                config_uri,
                "cem.module_map.module_imports_invalid",
                format!(
                    "module map `{map_uri}` {kind} `{identity}` `moduleImports` must be an object"
                ),
            ));
        }
    };
    Ok(ParsedModuleMapV3AssetEntry {
        path: path.to_owned(),
        content_type: content_type.to_owned(),
        module_imports,
    })
}

fn module_asset_manifest(
    mut assets: Vec<TransformGraphModuleAsset>,
) -> TransformGraphModuleAssetManifest {
    assets.sort_by(|left, right| {
        (
            &left.specifier,
            &left.target,
            &left.source_uri,
            &left.destination,
        )
            .cmp(&(
                &right.specifier,
                &right.target,
                &right.source_uri,
                &right.destination,
            ))
    });
    let contract_version = if assets.iter().any(|asset| {
        asset.source_byte_length != asset.byte_length || asset.source_sha256 != asset.sha256
    }) {
        2
    } else {
        1
    };
    let mut canonical = Vec::new();
    append_module_asset_hash_field(
        &mut canonical,
        if contract_version == 1 {
            b"cem-module-asset-manifest-v1"
        } else {
            b"cem-module-asset-manifest-v2"
        },
    );
    for asset in &assets {
        append_module_asset_hash_field(&mut canonical, asset.specifier.as_bytes());
        append_module_asset_hash_field(&mut canonical, asset.target.as_bytes());
        append_module_asset_hash_field(&mut canonical, asset.content_type.as_bytes());
        if contract_version >= 2 {
            append_module_asset_hash_field(
                &mut canonical,
                &asset.source_byte_length.to_be_bytes(),
            );
            append_module_asset_hash_field(&mut canonical, asset.source_sha256.as_bytes());
        }
        append_module_asset_hash_field(&mut canonical, &asset.byte_length.to_be_bytes());
        append_module_asset_hash_field(&mut canonical, asset.sha256.as_bytes());
    }
    TransformGraphModuleAssetManifest {
        contract_version,
        hash_scheme: "sha256".to_owned(),
        hash: crate::command_service::sha256_hex(&canonical),
        asset_count: assets.len() as u64,
        assets,
    }
}

fn append_module_asset_hash_field(canonical: &mut Vec<u8>, field: &[u8]) {
    canonical.extend_from_slice(&(field.len() as u64).to_be_bytes());
    canonical.extend_from_slice(field);
}

fn is_module_map_schema(schema: &str) -> bool {
    matches!(
        schema,
        MODULE_MAP_SCHEMA_URI | MODULE_MAP_V2_SCHEMA_URI | MODULE_MAP_V3_SCHEMA_URI
    )
}

fn validate_module_import_content_type(
    config_uri: &str,
    specifier: &str,
    path: &str,
    content_type: &str,
    v3: bool,
) -> Result<String, TransformGraphRequestError> {
    let content_type = content_type_essence(content_type);
    let expected_extensions: &[&str] = match content_type.as_str() {
        "text/javascript" => &["js", "mjs"],
        "application/json" if v3 => &["json"],
        _ => {
            return Err(config_error(
                config_uri,
                if v3 {
                    "cem.module_map.import_type_unsupported"
                } else {
                    "cem.module_map.asset_type_unsupported"
                },
                format!(
                    "module-map import `{specifier}` content type `{content_type}` is unsupported"
                ),
            ));
        }
    };
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if !extension
        .as_deref()
        .is_some_and(|extension| expected_extensions.contains(&extension))
    {
        return Err(config_error(
            config_uri,
            if v3 {
                "cem.module_map.import_type_mismatch"
            } else {
                "cem.module_map.asset_type_unsupported"
            },
            format!(
                "module-map import `{specifier}` path `{path}` does not match content type `{content_type}`"
            ),
        ));
    }
    Ok(content_type)
}

fn validate_module_import_target(
    config_uri: &str,
    specifier: &str,
    target: &str,
    content_type: &str,
) -> Result<(), TransformGraphRequestError> {
    let relative = target.strip_prefix("./").ok_or_else(|| {
        config_error(
            config_uri,
            "cem.module_map.target_invalid",
            format!(
                "module-map import `{specifier}` target `{target}` must be an app-relative `./` URL"
            ),
        )
    })?;
    let path = Path::new(relative);
    let contained = !relative.is_empty()
        && !target
            .chars()
            .any(|character| matches!(character, '\\' | '?' | '#'))
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)));
    if !contained {
        return Err(config_error(
            config_uri,
            "cem.module_map.target_invalid",
            format!(
                "module-map import `{specifier}` target `{target}` must stay within the app output"
            ),
        ));
    }
    validate_module_import_content_type(config_uri, specifier, target, content_type, true)
        .map(|_| ())
}

#[derive(Debug)]
struct JavaScriptModuleSpecifier {
    start: usize,
    end: usize,
    value: String,
    escaped: bool,
}

#[derive(Debug)]
enum JavaScriptTokenKind {
    Identifier(String),
    String { value: String, escaped: bool },
    Punct(u8),
    Other,
}

#[derive(Debug)]
struct JavaScriptToken {
    kind: JavaScriptTokenKind,
    start: usize,
    end: usize,
}

fn rewrite_declared_module_specifiers(
    config_uri: &str,
    asset_identity: &str,
    asset_target: &str,
    source: &[u8],
    declared: &BTreeMap<String, String>,
    deployment_assets: &BTreeMap<String, (String, String)>,
) -> Result<Vec<u8>, TransformGraphRequestError> {
    let source_text = std::str::from_utf8(source).map_err(|_| {
        config_error(
            config_uri,
            "cem.module_map.javascript_utf8_invalid",
            format!("module-map JavaScript asset `{asset_identity}` must be valid UTF-8"),
        )
    })?;
    for (specifier, target_identity) in declared {
        validate_module_specifier(config_uri, specifier)?;
        let Some((_, content_type)) = deployment_assets.get(target_identity) else {
            return Err(config_error(
                config_uri,
                "cem.module_map.module_import_target_missing",
                format!(
                    "module-map JavaScript asset `{asset_identity}` maps `{specifier}` to undeclared asset `{target_identity}`"
                ),
            ));
        };
        if !matches!(content_type.as_str(), "text/javascript" | "application/json") {
            return Err(config_error(
                config_uri,
                "cem.module_map.module_import_target_invalid",
                format!(
                    "module-map JavaScript asset `{asset_identity}` maps `{specifier}` to non-module asset `{target_identity}` with content type `{content_type}`"
                ),
            ));
        }
    }

    let specifiers = javascript_module_specifiers(source_text);
    let mut used = BTreeSet::new();
    let mut replacements = Vec::new();
    for specifier in specifiers {
        if !is_bare_javascript_specifier(&specifier.value) {
            continue;
        }
        if specifier.escaped {
            return Err(config_error(
                config_uri,
                "cem.module_map.javascript_specifier_invalid",
                format!(
                    "module-map JavaScript asset `{asset_identity}` bare specifier uses unsupported escapes"
                ),
            ));
        }
        let Some(target_identity) = declared.get(&specifier.value) else {
            return Err(config_error(
                config_uri,
                "cem.module_map.javascript_bare_specifier_undeclared",
                format!(
                    "module-map JavaScript asset `{asset_identity}` uses undeclared bare specifier `{}`",
                    specifier.value
                ),
            ));
        };
        let (target, _) = &deployment_assets[target_identity];
        let replacement = relative_module_target(asset_target, target);
        used.insert(specifier.value.clone());
        replacements.push((specifier.start, specifier.end, replacement));
    }
    if let Some(unused) = declared.keys().find(|specifier| !used.contains(*specifier)) {
        return Err(config_error(
            config_uri,
            "cem.module_map.javascript_rewrite_unused",
            format!(
                "module-map JavaScript asset `{asset_identity}` declares unused module import `{unused}`"
            ),
        ));
    }

    let mut output = source.to_vec();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.splice(start..end, replacement.bytes());
    }
    Ok(output)
}

fn javascript_module_specifiers(source: &str) -> Vec<JavaScriptModuleSpecifier> {
    let tokens = javascript_tokens(source.as_bytes());
    let mut specifiers = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let JavaScriptTokenKind::Identifier(keyword) = &token.kind else {
            continue;
        };
        if !matches!(keyword.as_str(), "import" | "export")
            || index
                .checked_sub(1)
                .and_then(|previous| tokens.get(previous))
                .is_some_and(|previous| matches!(previous.kind, JavaScriptTokenKind::Punct(b'.')))
        {
            continue;
        }
        let Some(next) = tokens.get(index + 1) else {
            continue;
        };
        if keyword == "import" {
            match &next.kind {
                JavaScriptTokenKind::String { .. } => {
                    push_javascript_module_specifier(&mut specifiers, next)
                }
                JavaScriptTokenKind::Punct(b'(') => {
                    if let Some(argument) = tokens.get(index + 2) {
                        push_javascript_module_specifier(&mut specifiers, argument);
                    }
                }
                JavaScriptTokenKind::Punct(b'.' | b':') => {}
                _ => find_from_module_specifier(&tokens, index + 1, &mut specifiers),
            }
        } else {
            find_from_module_specifier(&tokens, index + 1, &mut specifiers);
        }
    }
    specifiers.sort_by_key(|specifier| specifier.start);
    specifiers.dedup_by_key(|specifier| specifier.start);
    specifiers
}

fn find_from_module_specifier(
    tokens: &[JavaScriptToken],
    start: usize,
    output: &mut Vec<JavaScriptModuleSpecifier>,
) {
    for (offset, token) in tokens[start..].iter().enumerate() {
        if matches!(token.kind, JavaScriptTokenKind::Punct(b';')) {
            break;
        }
        if matches!(&token.kind, JavaScriptTokenKind::Identifier(value) if value == "from") {
            if let Some(specifier) = tokens.get(start + offset + 1) {
                push_javascript_module_specifier(output, specifier);
            }
            break;
        }
    }
}

fn push_javascript_module_specifier(
    output: &mut Vec<JavaScriptModuleSpecifier>,
    token: &JavaScriptToken,
) {
    if let JavaScriptTokenKind::String { value, escaped } = &token.kind {
        output.push(JavaScriptModuleSpecifier {
            start: token.start + 1,
            end: token.end - 1,
            value: value.clone(),
            escaped: *escaped,
        });
    }
}

fn javascript_tokens(source: &[u8]) -> Vec<JavaScriptToken> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < source.len() {
        let byte = source[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if byte == b'/' && source.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < source.len() && !matches!(source[index], b'\n' | b'\r') {
                index += 1;
            }
            continue;
        }
        if byte == b'/' && source.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < source.len()
                && !(source[index] == b'*' && source[index + 1] == b'/')
            {
                index += 1;
            }
            index = (index + 2).min(source.len());
            continue;
        }
        if byte == b'/' && javascript_regex_can_start(tokens.last()) {
            let start = index;
            index += 1;
            let mut character_class = false;
            while index < source.len() {
                if source[index] == b'\\' {
                    index = (index + 2).min(source.len());
                    continue;
                }
                if source[index] == b'[' {
                    character_class = true;
                } else if source[index] == b']' {
                    character_class = false;
                } else if source[index] == b'/' && !character_class {
                    index += 1;
                    while index < source.len() && is_javascript_identifier_continue(source[index]) {
                        index += 1;
                    }
                    break;
                }
                index += 1;
            }
            tokens.push(JavaScriptToken {
                kind: JavaScriptTokenKind::Other,
                start,
                end: index,
            });
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            let quote = byte;
            let start = index;
            index += 1;
            let content_start = index;
            let mut escaped = false;
            while index < source.len() {
                if source[index] == b'\\' {
                    escaped = true;
                    index = (index + 2).min(source.len());
                    continue;
                }
                if source[index] == quote {
                    break;
                }
                index += 1;
            }
            let content_end = index.min(source.len());
            index = (index + 1).min(source.len());
            tokens.push(JavaScriptToken {
                kind: JavaScriptTokenKind::String {
                    value: String::from_utf8_lossy(&source[content_start..content_end]).into_owned(),
                    escaped,
                },
                start,
                end: index,
            });
            continue;
        }
        if byte == b'`' {
            let start = index;
            index += 1;
            while index < source.len() {
                if source[index] == b'\\' {
                    index = (index + 2).min(source.len());
                    continue;
                }
                if source[index] == b'`' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            tokens.push(JavaScriptToken {
                kind: JavaScriptTokenKind::Other,
                start,
                end: index,
            });
            continue;
        }
        if is_javascript_identifier_start(byte) {
            let start = index;
            index += 1;
            while index < source.len() && is_javascript_identifier_continue(source[index]) {
                index += 1;
            }
            tokens.push(JavaScriptToken {
                kind: JavaScriptTokenKind::Identifier(
                    String::from_utf8_lossy(&source[start..index]).into_owned(),
                ),
                start,
                end: index,
            });
            continue;
        }
        tokens.push(JavaScriptToken {
            kind: if byte.is_ascii_punctuation() {
                JavaScriptTokenKind::Punct(byte)
            } else {
                JavaScriptTokenKind::Other
            },
            start: index,
            end: index + 1,
        });
        index += 1;
    }
    tokens
}

fn javascript_regex_can_start(previous: Option<&JavaScriptToken>) -> bool {
    match previous.map(|token| &token.kind) {
        None => true,
        Some(JavaScriptTokenKind::Punct(b')' | b']' | b'}' | b'.')) => false,
        Some(JavaScriptTokenKind::Punct(_)) => true,
        Some(JavaScriptTokenKind::Identifier(value)) => matches!(
            value.as_str(),
            "await"
                | "case"
                | "delete"
                | "do"
                | "else"
                | "in"
                | "instanceof"
                | "new"
                | "return"
                | "throw"
                | "typeof"
                | "void"
                | "yield"
        ),
        Some(JavaScriptTokenKind::String { .. } | JavaScriptTokenKind::Other) => false,
    }
}

fn is_javascript_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$') || !byte.is_ascii()
}

fn is_javascript_identifier_continue(byte: u8) -> bool {
    is_javascript_identifier_start(byte) || byte.is_ascii_digit()
}

fn is_bare_javascript_specifier(specifier: &str) -> bool {
    !specifier.is_empty()
        && !specifier.starts_with('.')
        && !specifier.starts_with('/')
        && !specifier.starts_with('#')
        && uri_scheme(specifier).is_none()
}

fn relative_module_target(from: &str, to: &str) -> String {
    let from_parts = from
        .strip_prefix("./")
        .unwrap_or(from)
        .split('/')
        .collect::<Vec<_>>();
    let to_parts = to
        .strip_prefix("./")
        .unwrap_or(to)
        .split('/')
        .collect::<Vec<_>>();
    let from_directory = &from_parts[..from_parts.len().saturating_sub(1)];
    let common = from_directory
        .iter()
        .zip(&to_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; from_directory.len().saturating_sub(common)];
    parts.extend_from_slice(&to_parts[common..]);
    let relative = parts.join("/");
    if relative.starts_with("../") {
        relative
    } else {
        format!("./{relative}")
    }
}

fn validate_module_resource_content_type(
    config_uri: &str,
    name: &str,
    path: &str,
    content_type: &str,
) -> Result<String, TransformGraphRequestError> {
    let content_type = content_type_essence(content_type);
    let expected_extensions: &[&str] = match content_type.as_str() {
        "text/javascript" => &["js", "mjs"],
        "text/css" => &["css"],
        "application/wasm" => &["wasm"],
        _ => {
            return Err(config_error(
                config_uri,
                "cem.module_map.resource_type_unsupported",
                format!(
                    "module-map resource `{name}` content type `{content_type}` is unsupported; expected `text/javascript`, `text/css`, or `application/wasm`"
                ),
            ));
        }
    };
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if !extension
        .as_deref()
        .is_some_and(|extension| expected_extensions.contains(&extension))
    {
        return Err(config_error(
            config_uri,
            "cem.module_map.resource_type_mismatch",
            format!(
                "module-map resource `{name}` path `{path}` does not match content type `{content_type}`"
            ),
        ));
    }
    Ok(content_type)
}

fn validate_module_resource_target(
    config_uri: &str,
    name: &str,
    target: &str,
    content_type: &str,
) -> Result<(), TransformGraphRequestError> {
    let relative = target.strip_prefix("./").ok_or_else(|| {
        config_error(
            config_uri,
            "cem.module_map.resource_target_invalid",
            format!(
                "module-map resource `{name}` target `{target}` must be an app-relative `./` URL"
            ),
        )
    })?;
    let path = Path::new(relative);
    let has_only_normal_segments = path
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_)));
    if relative.is_empty()
        || target
            .chars()
            .any(|character| matches!(character, '\\' | '?' | '#'))
        || !has_only_normal_segments
    {
        return Err(config_error(
            config_uri,
            "cem.module_map.resource_target_invalid",
            format!(
                "module-map resource `{name}` target `{target}` must stay within the app output"
            ),
        ));
    }
    validate_module_resource_content_type(config_uri, name, target, content_type).map(|_| ())
}

fn validate_module_specifier(
    config_uri: &str,
    specifier: &str,
) -> Result<(), TransformGraphRequestError> {
    let specifier = specifier.trim();
    if specifier.is_empty()
        || specifier.starts_with('.')
        || specifier.starts_with('/')
        || specifier.starts_with('#')
        || specifier.ends_with('/')
        || uri_scheme(specifier).is_some()
    {
        return Err(config_error(
            config_uri,
            "cem.module_map.specifier_invalid",
            format!(
                "module-map import key `{specifier}` must be an exact bare npm module specifier"
            ),
        ));
    }
    Ok(())
}

fn module_asset_export_is_html(scope: &ScopeConfig) -> bool {
    scope
        .default_content_type
        .as_deref()
        .map(|content_type| {
            content_type
                .split(';')
                .next()
                .unwrap_or(content_type)
                .trim()
                .to_ascii_lowercase()
        })
        .is_some_and(|content_type| {
            matches!(content_type.as_str(), "text/html" | "application/xhtml+xml")
        })
        || matches!(
            scope.schema.as_deref(),
            Some("https://cem.dev/ns/data/html/1" | "https://cem.dev/ns/data/xhtml/1")
        )
}

fn variant_id(base: &str, index: usize, count: usize) -> String {
    if count == 1 {
        base.to_owned()
    } else {
        format!("{base}:{index}")
    }
}

fn expand_binding_template(
    template: &str,
    bindings: &BTreeMap<String, String>,
    config_uri: &str,
    label: &str,
    code_prefix: &str,
) -> Result<String, TransformGraphRequestError> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            return Err(config_error(
                config_uri,
                &format!("cem.transform_config.{code_prefix}_unclosed"),
                format!("{label} `{template}` has an unclosed binding"),
            ));
        };
        let name = &after_open[..close];
        if name.trim().is_empty() {
            return Err(config_error(
                config_uri,
                &format!("cem.transform_config.{code_prefix}_empty"),
                format!("{label} `{template}` has an empty binding"),
            ));
        }
        let Some(value) = bindings.get(name) else {
            return Err(config_error(
                config_uri,
                &format!("cem.transform_config.{code_prefix}_unknown"),
                format!("{label} `{template}` references unknown binding `{name}`"),
            ));
        };
        out.push_str(value);
        rest = &after_open[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn stage_params(
    node: &TransformGraphNode,
    bindings: &BTreeMap<String, String>,
    config_uri: &str,
) -> Result<BTreeMap<String, serde_json::Value>, TransformGraphRequestError> {
    node.params
        .iter()
        .map(|(name, value)| {
            expand_binding_template(
                value,
                bindings,
                config_uri,
                "param value template",
                "param_binding",
            )
            .map(|value| (name.clone(), serde_json::Value::String(value)))
        })
        .collect()
}

fn template_entrypoint(value: Option<&str>) -> TransformTemplateEntrypoint {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(TransformTemplateEntrypoint::named)
        .unwrap_or_else(TransformTemplateEntrypoint::implicit)
}

fn validate_template_surface(
    template_kind: TransformTemplateKind,
    entrypoint: &TransformTemplateEntrypoint,
    params: &BTreeMap<String, serde_json::Value>,
    config_uri: &str,
) -> Result<(), TransformGraphRequestError> {
    let message =
        if template_kind == TransformTemplateKind::CemQlExpression && !entrypoint.is_implicit() {
            Some("CEM-QL expression transforms do not accept a named entrypoint")
        } else if template_kind == TransformTemplateKind::XPath
            && (!entrypoint.is_implicit() || !params.is_empty())
        {
            Some("XPath transforms require the implicit entrypoint and no params")
        } else if !matches!(
            template_kind,
            TransformTemplateKind::CemNative
                | TransformTemplateKind::Xslt
                | TransformTemplateKind::CemQlExpression
        ) && (!entrypoint.is_implicit() || !params.is_empty())
        {
            Some("this transform adapter does not support entrypoints or params")
        } else {
            None
        };
    match message {
        Some(message) => Err(config_error(
            config_uri,
            "cem.transform_config.template_surface",
            message,
        )),
        None => Ok(()),
    }
}

fn transform_execution_policy(
    template_kind: TransformTemplateKind,
    entrypoint: &TransformTemplateEntrypoint,
    params: &BTreeMap<String, serde_json::Value>,
) -> TransformExecutionPolicy {
    TransformExecutionPolicy {
        runtime_phase: match template_kind {
            TransformTemplateKind::Xslt => TransformRuntimePhase::XsltParity,
            TransformTemplateKind::CemQlExpression => TransformRuntimePhase::CemQlExpression,
            TransformTemplateKind::CemNative if !entrypoint.is_implicit() || !params.is_empty() => {
                TransformRuntimePhase::CemNativeModules
            }
            TransformTemplateKind::XPath => TransformRuntimePhase::XPath,
            TransformTemplateKind::CemNative => TransformRuntimePhase::CemQlFragment,
        },
        ..TransformExecutionPolicy::default()
    }
}

fn primary_ref(
    graph: &TransformGraphConfig,
    node: &TransformGraphNode,
    config_uri: &str,
) -> Result<(String, TransformGraphEdgeRole), TransformGraphRequestError> {
    if let Some(input_ref) = node.input_ref.as_ref() {
        return Ok((input_ref.clone(), TransformGraphEdgeRole::Input));
    }
    graph
        .edges
        .iter()
        .find(|edge| {
            edge.to == node.id
                && matches!(
                    edge.role,
                    TransformGraphEdgeRole::Input | TransformGraphEdgeRole::Parent
                )
        })
        .map(|edge| (edge.from.clone(), edge.role))
        .ok_or_else(|| {
            config_error(
                config_uri,
                "cem.transform_config.input_missing",
                format!("transform graph node `{}` requires an input edge", node.id),
            )
        })
}

fn variants_for_ref(
    variants: &BTreeMap<String, Vec<ArtifactVariant>>,
    owner_id: &str,
    field: &str,
    target: &str,
    config_uri: &str,
) -> Result<Vec<ArtifactVariant>, TransformGraphRequestError> {
    variants.get(target).cloned().ok_or_else(|| {
        config_error(
            config_uri,
            "cem.transform_config.artifact_reference_unknown",
            format!(
                "transform graph node `{owner_id}` references unknown artifact `{target}` via `{field}`"
            ),
        )
    })
}

fn single_variant_for_ref(
    variants: &BTreeMap<String, Vec<ArtifactVariant>>,
    owner_id: &str,
    field: &str,
    target: &str,
    config_uri: &str,
) -> Result<ArtifactVariant, TransformGraphRequestError> {
    let matches = variants_for_ref(variants, owner_id, field, target, config_uri)?;
    if matches.len() != 1 {
        return Err(config_error(
            config_uri,
            "cem.transform_config.join_multi_artifact_unsupported",
            format!(
                "node `{owner_id}` references multi-artifact `{target}` via `{field}`; explicit join semantics are not implemented"
            ),
        ));
    }
    Ok(matches[0].clone())
}

fn engine_dependency_role(role: TransformGraphEdgeRole) -> TransformGraphDependencyRole {
    match role {
        TransformGraphEdgeRole::Parent => TransformGraphDependencyRole::Parent,
        TransformGraphEdgeRole::Input => TransformGraphDependencyRole::PrimaryInput,
        TransformGraphEdgeRole::With => TransformGraphDependencyRole::SecondaryInput,
    }
}

fn engine_join_mode(mode: ConfigJoinMode) -> TransformGraphJoinMode {
    match mode {
        ConfigJoinMode::Collect => TransformGraphJoinMode::Collect,
        ConfigJoinMode::GroupBy => TransformGraphJoinMode::GroupBy,
        ConfigJoinMode::MatchBy => TransformGraphJoinMode::MatchBy,
        ConfigJoinMode::Zip => TransformGraphJoinMode::Zip,
    }
}

fn engine_importmap_rewrite_mode(
    mode: transform_config::TransformGraphImportMapRewriteMode,
) -> TransformGraphImportMapRewriteMode {
    match mode {
        transform_config::TransformGraphImportMapRewriteMode::ReplaceImports => {
            TransformGraphImportMapRewriteMode::ReplaceImports
        }
        transform_config::TransformGraphImportMapRewriteMode::Merge => {
            TransformGraphImportMapRewriteMode::Merge
        }
        transform_config::TransformGraphImportMapRewriteMode::ReplaceScript => {
            TransformGraphImportMapRewriteMode::ReplaceScript
        }
    }
}

fn engine_importmap_missing_policy(
    policy: transform_config::TransformGraphImportMapMissingPolicy,
) -> TransformGraphImportMapMissingPolicy {
    match policy {
        transform_config::TransformGraphImportMapMissingPolicy::Error => {
            TransformGraphImportMapMissingPolicy::Error
        }
        transform_config::TransformGraphImportMapMissingPolicy::Ignore => {
            TransformGraphImportMapMissingPolicy::Ignore
        }
        transform_config::TransformGraphImportMapMissingPolicy::Insert => {
            TransformGraphImportMapMissingPolicy::Insert
        }
    }
}

fn join_by(
    node: &TransformGraphNode,
    config_uri: &str,
) -> Result<String, TransformGraphRequestError> {
    let by = node.join_by.as_deref().unwrap_or("").trim();
    if by.is_empty() {
        return Err(config_error(
            config_uri,
            "cem.transform_config.join_by_missing",
            format!("join node `{}` with keyed `@mode` requires `@by`", node.id),
        ));
    }
    if matches!(by, "count" | "key") {
        return Err(config_error(
            config_uri,
            "cem.transform_config.join_by_reserved",
            format!(
                "join node `{}` uses reserved grouping binding `{by}`",
                node.id
            ),
        ));
    }
    Ok(by.to_owned())
}

fn group_by_join_groups(
    node: &TransformGraphNode,
    input_variants: Vec<ArtifactVariant>,
    config_uri: &str,
) -> Result<Vec<JoinGroup>, TransformGraphRequestError> {
    let by = join_by(node, config_uri)?;
    let mut groups: BTreeMap<String, Vec<ArtifactVariant>> = BTreeMap::new();
    for variant in input_variants {
        let value = variant.bindings.get(&by).ok_or_else(|| {
            config_error(
                config_uri,
                "cem.transform_config.join_by_unknown",
                format!(
                    "join node `{}` groups by unknown binding `{by}` on artifact `{}`",
                    node.id, variant.id
                ),
            )
        })?;
        groups.entry(value.clone()).or_default().push(variant);
    }
    let count = groups.len();
    Ok(groups
        .into_iter()
        .enumerate()
        .map(|(index, (key, variants))| {
            let id = variant_id(&node.id, index, count);
            let mut bindings = BTreeMap::from([
                ("count".to_owned(), variants.len().to_string()),
                ("key".to_owned(), key.clone()),
            ]);
            bindings.insert(by.clone(), key);
            let inputs = variants
                .into_iter()
                .map(|variant| ("primary".to_owned(), variant))
                .collect();
            (id, inputs, bindings)
        })
        .collect())
}

fn match_by_join_groups(
    node: &TransformGraphNode,
    primary_variants: Vec<ArtifactVariant>,
    variants: &BTreeMap<String, Vec<ArtifactVariant>>,
    config_uri: &str,
) -> Result<Vec<JoinGroup>, TransformGraphRequestError> {
    let by = join_by(node, config_uri)?;
    if node.with.is_empty() {
        return Err(config_error(
            config_uri,
            "cem.transform_config.join_with_missing",
            format!(
                "join node `{}` with `@mode=\"match-by\"` requires at least one `@with:*` input",
                node.id
            ),
        ));
    }
    let mut primary_groups: BTreeMap<String, Vec<ArtifactVariant>> = BTreeMap::new();
    for variant in primary_variants {
        let value = variant.bindings.get(&by).ok_or_else(|| {
            config_error(
                config_uri,
                "cem.transform_config.join_by_unknown",
                format!(
                    "join node `{}` matches by unknown binding `{by}` on artifact `{}`",
                    node.id, variant.id
                ),
            )
        })?;
        primary_groups
            .entry(value.clone())
            .or_default()
            .push(variant);
    }
    let mut secondary_groups = BTreeMap::new();
    for (name, target) in &node.with {
        let secondary = variants_for_ref(
            variants,
            &node.id,
            &format!("with:{name}"),
            target,
            config_uri,
        )?;
        let mut by_key: BTreeMap<String, Vec<ArtifactVariant>> = BTreeMap::new();
        for variant in secondary {
            let value = variant.bindings.get(&by).ok_or_else(|| {
                config_error(
                    config_uri,
                    "cem.transform_config.join_by_unknown",
                    format!(
                        "join node `{}` matches by unknown binding `{by}` on artifact `{}`",
                        node.id, variant.id
                    ),
                )
            })?;
            by_key.entry(value.clone()).or_default().push(variant);
        }
        secondary_groups.insert(name.clone(), by_key);
    }
    let count = primary_groups.len();
    Ok(primary_groups
        .into_iter()
        .enumerate()
        .map(|(index, (key, primary))| {
            let id = variant_id(&node.id, index, count);
            let mut inputs = primary
                .into_iter()
                .map(|variant| ("primary".to_owned(), variant))
                .collect::<Vec<_>>();
            for (name, by_key) in &secondary_groups {
                if let Some(matches) = by_key.get(&key) {
                    inputs.extend(
                        matches
                            .iter()
                            .cloned()
                            .map(|variant| (name.clone(), variant)),
                    );
                }
            }
            let mut bindings = BTreeMap::from([
                ("count".to_owned(), inputs.len().to_string()),
                ("key".to_owned(), key.clone()),
            ]);
            bindings.insert(by.clone(), key);
            (id, inputs, bindings)
        })
        .collect())
}

fn zip_join_groups(
    node: &TransformGraphNode,
    primary_variants: Vec<ArtifactVariant>,
    variants: &BTreeMap<String, Vec<ArtifactVariant>>,
    config_uri: &str,
) -> Result<Vec<JoinGroup>, TransformGraphRequestError> {
    if node.with.is_empty() {
        return Err(config_error(
            config_uri,
            "cem.transform_config.join_with_missing",
            format!(
                "join node `{}` with `@mode=\"zip\"` requires at least one `@with:*` input",
                node.id
            ),
        ));
    }
    let count = primary_variants.len();
    let mut secondary_inputs = BTreeMap::new();
    for (name, target) in &node.with {
        let secondary = variants_for_ref(
            variants,
            &node.id,
            &format!("with:{name}"),
            target,
            config_uri,
        )?;
        if secondary.len() != count {
            return Err(config_error(
                config_uri,
                "cem.transform_config.join_zip_count_mismatch",
                format!(
                    "join node `{}` cannot zip primary input count {} with `@with:{name}` count {}; zip joins require equal input counts",
                    node.id,
                    count,
                    secondary.len()
                ),
            ));
        }
        secondary_inputs.insert(name.clone(), secondary);
    }
    Ok(primary_variants
        .into_iter()
        .enumerate()
        .map(|(index, primary)| {
            let id = variant_id(&node.id, index, count);
            let mut inputs = vec![("primary".to_owned(), primary)];
            for (name, secondary) in &secondary_inputs {
                inputs.push((name.clone(), secondary[index].clone()));
            }
            let bindings = BTreeMap::from([
                ("count".to_owned(), inputs.len().to_string()),
                ("index".to_owned(), index.to_string()),
            ]);
            (id, inputs, bindings)
        })
        .collect())
}

fn apply_stage_targets(stages: &mut [TransformGraphStage], exports: &[TransformGraphExport]) {
    for stage in stages {
        stage.target = export_target_for_stage(&stage.id, exports);
    }
}

fn export_target_for_stage(
    stage_id: &str,
    exports: &[TransformGraphExport],
) -> Option<FormatIdentity> {
    let mut target = None;
    for export in exports.iter().filter(|export| export.input == stage_id) {
        let Some(candidate) = export.target.clone() else {
            continue;
        };
        match &target {
            None => target = Some(candidate),
            Some(existing) if existing == &candidate => {}
            Some(_) => return None,
        }
    }
    target
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::transform_config::{parse_transform_graph_config, TransformGraphParseRequest};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct FixtureDir(PathBuf);

    impl FixtureDir {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cem-ml-transform-graph-provider-parity-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(path.join("inputs")).expect("fixture input directory");
            fs::create_dir_all(path.join("templates")).expect("fixture template directory");
            fs::create_dir_all(path.join("maps")).expect("fixture map directory");
            Self(path)
        }

        fn write(&self, relative: &str, bytes: &[u8]) -> String {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture parent");
            }
            fs::write(&path, bytes).expect("fixture write");
            path.display().to_string()
        }
    }

    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn graph(config_uri: &str, bytes: &[u8]) -> TransformGraphConfig {
        parse_transform_graph_config(TransformGraphParseRequest {
            bytes: bytes.to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/cem+xml".to_owned()),
                schema: Some(transform_config::TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
                base_uri: Some(config_uri.to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: Some(config_uri.to_owned()),
        })
        .expect("fixture graph parses")
        .graph
    }

    fn assert_request_parts_equal(left: &TransformGraphRequest, right: &TransformGraphRequest) {
        assert_eq!(
            format!("{:#?}", left.imports),
            format!("{:#?}", right.imports)
        );
        assert_eq!(format!("{:#?}", left.joins), format!("{:#?}", right.joins));
        assert_eq!(
            format!("{:#?}", left.stages),
            format!("{:#?}", right.stages)
        );
        assert_eq!(
            format!("{:#?}", left.importmap_rewrites),
            format!("{:#?}", right.importmap_rewrites)
        );
        assert_eq!(
            format!("{:#?}", left.exports),
            format!("{:#?}", right.exports)
        );
        assert_eq!(left.edges, right.edges);
        assert_eq!(left.preserve_source_offsets, right.preserve_source_offsets);
        assert_eq!(left.execution_policy, right.execution_policy);
    }

    #[test]
    fn manifest_and_filesystem_providers_lower_equivalent_graph_requests() {
        let fixture = FixtureDir::new();
        let graph_bytes = br#"{@doc cem-ml 1}{run | {import @id=data @src="inputs/*.cem" @content-type="application/cem+xml" | {join @id=joined @mode=collect | {transform @id=view @src="templates/main.cemt" @template-content-type="application/vnd.cem.template+cem" | {param @name=source-count @value="{count}"}{rewrite-importmap @id=rewritten @target-map="maps/target.json" | {export @id=output @out="dist/catalog.cem" @content-type="application/cem+xml"}}}}}}"#;
        let config_uri = fixture.write("graph.cem", graph_bytes);
        let input_a = fixture.write("inputs/a.cem", b"{a}");
        let input_b = fixture.write("inputs/b.cem", b"{b}");
        let template = fixture.write("templates/main.cemt", b"{template}");
        let target_map = fixture.write(
            "maps/target.json",
            br#"{"imports":{"catalog":"./catalog.cem"}}"#,
        );
        let graph = graph(&config_uri, graph_bytes);
        let resources = BTreeMap::from([
            (
                input_a,
                VirtualResourceV1 {
                    bytes: b"{a}".to_vec(),
                    identity: None,
                },
            ),
            (
                input_b,
                VirtualResourceV1 {
                    bytes: b"{b}".to_vec(),
                    identity: None,
                },
            ),
            (
                template,
                VirtualResourceV1 {
                    bytes: b"{template}".to_vec(),
                    identity: None,
                },
            ),
            (
                target_map,
                VirtualResourceV1 {
                    bytes: br#"{"imports":{"catalog":"./catalog.cem"}}"#.to_vec(),
                    identity: None,
                },
            ),
        ]);
        let context = EngineContext::default();
        let manifest = ManifestTransformGraphResourceProvider::new(&config_uri, &resources);
        let filesystem = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);
        let manifest_request =
            lower_transform_graph_request(&context, &graph, &manifest, &config_uri, true)
                .expect("manifest lowering");
        let filesystem_request =
            lower_transform_graph_request(&context, &graph, &filesystem, &config_uri, true)
                .expect("filesystem lowering");

        assert_request_parts_equal(&manifest_request, &filesystem_request);
        assert_eq!(manifest_request.imports.len(), 2);
        assert_eq!(manifest_request.joins.len(), 1);
        assert_eq!(manifest_request.stages.len(), 1);
        assert_eq!(manifest_request.importmap_rewrites.len(), 1);
        assert_eq!(manifest_request.exports.len(), 1);
        assert_eq!(
            manifest_request.exports[0]
                .target_scope
                .default_content_type
                .as_deref(),
            Some("application/cem+xml")
        );
        assert!(manifest_request.exports[0].target_scope.schema.is_none());
        assert_eq!(
            manifest_request.stages[0].params["source-count"],
            serde_json::json!("2")
        );
        assert_eq!(manifest_request.imports[0].scheduler_scope_id, 0);
        assert_eq!(manifest_request.exports[0].scheduler_scope_id, 6);
    }

    #[test]
    fn regular_opaque_import_lowers_as_byte_preserving_graph_input() {
        let fixture = FixtureDir::new();
        let graph_bytes = br#"{run | {import @id=asset @src="inputs/asset.bin" @content-type="application/octet-stream" @opaque=true | {export @id=copy @out="dist/asset.bin" @content-type="application/octet-stream"}}}"#;
        let config_uri = fixture.write("graph.cem", graph_bytes);
        let source_uri = fixture.write("inputs/asset.bin", &[0x00, 0xff, 0x61, 0x73, 0x6d, 0x00]);
        let graph = graph(&config_uri, graph_bytes);
        let context = EngineContext::default();
        let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);

        let request = lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
            .expect("opaque import lowers");

        assert_eq!(request.imports.len(), 1);
        assert!(request.imports[0].opaque);
        assert_eq!(request.imports[0].input.uri, source_uri);
        assert_eq!(
            request.imports[0].input.bytes,
            vec![0x00, 0xff, 0x61, 0x73, 0x6d, 0x00]
        );
        assert_eq!(request.exports.len(), 1);
        assert_eq!(request.exports[0].input, request.imports[0].id);
    }

    #[test]
    fn dedicated_module_maps_lower_declared_javascript_as_graph_artifacts() {
        let fixture = FixtureDir::new();
        let graph_bytes = br#"{run | {import @id=page @src="src/page.html" @content-type="text/html" | {rewrite-importmap @id=modules @source-map="maps/source.module-map.json" @target-map="maps/dist.module-map.json" | {export @id=html @out="dist/page.html" @content-type="text/html"}}}}"#;
        let config_uri = fixture.write("graph.cem", graph_bytes);
        fixture.write(
            "src/page.html",
            br#"<script type="importmap">{"imports":{"@pkg/runtime":"../node_modules/@pkg/runtime.js"}}</script>"#,
        );
        fixture.write(
            "maps/source.module-map.json",
            br#"{"$schema":"https://cem.dev/ns/data/module-map/1","imports":{"@pkg/runtime":"../node_modules/@pkg/runtime.js"}}"#,
        );
        fixture.write(
            "maps/dist.module-map.json",
            br#"{"$schema":"https://cem.dev/ns/data/module-map/1","imports":{"@pkg/runtime":"./vendor/runtime.js"}}"#,
        );
        let module_source = fixture.write(
            "node_modules/@pkg/runtime.js",
            b"export const runtime = 'declared';\n",
        );
        let graph = graph(&config_uri, graph_bytes);
        let context = EngineContext::default();
        let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);

        let request = lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
            .expect("dedicated module maps lower");

        assert_eq!(request.imports.len(), 2);
        let module_import = request
            .imports
            .iter()
            .find(|import| import.input.uri == module_source)
            .expect("declared module graph import");
        assert_eq!(
            module_import
                .input
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/javascript")
        );
        let module_export = request
            .exports
            .iter()
            .find(|export| export.input == module_import.id)
            .expect("declared module graph export");
        let expected_destination = fixture.0.join("dist/vendor/runtime.js");
        assert_eq!(
            module_export.destination.as_deref(),
            Some(expected_destination.to_string_lossy().as_ref())
        );
        assert_eq!(
            request.importmap_rewrites[0].target_imports["@pkg/runtime"],
            "./vendor/runtime.js"
        );
        assert_eq!(request.module_asset_manifest.asset_count, 1);
        assert_eq!(request.module_asset_manifest.hash_scheme, "sha256");
        assert_eq!(request.module_asset_manifest.hash.len(), 64);
        let asset = &request.module_asset_manifest.assets[0];
        assert_eq!(asset.specifier, "@pkg/runtime");
        assert_eq!(
            asset.source_map_uri,
            fixture
                .0
                .join("maps/source.module-map.json")
                .to_string_lossy()
        );
        assert_eq!(asset.source_uri, module_source);
        assert_eq!(asset.target, "./vendor/runtime.js");
        assert_eq!(
            asset.destination,
            expected_destination.to_string_lossy().as_ref()
        );
        assert_eq!(asset.content_type, "text/javascript");
        assert_eq!(asset.byte_length, 35);
        assert_eq!(
            asset.sha256,
            crate::command_service::sha256_hex(b"export const runtime = 'declared';\n")
        );

        let repeated =
            lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
                .expect("repeated dedicated module-map lowering");
        assert_eq!(
            repeated.module_asset_manifest,
            request.module_asset_manifest
        );

        fixture.write(
            "node_modules/@pkg/runtime.js",
            b"export const runtime = 'different';\n",
        );
        let changed = lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
            .expect("changed dedicated module-map lowering");
        assert_ne!(
            changed.module_asset_manifest.hash,
            request.module_asset_manifest.hash
        );
    }

    #[test]
    fn dedicated_module_map_v2_lowers_explicit_resources_as_graph_artifacts() {
        let fixture = FixtureDir::new();
        let graph_bytes = br#"{run | {import @id=page @src="src/page.html" @content-type="text/html" | {rewrite-importmap @id=modules @source-map="maps/source.module-map.json" @target-map="maps/dist.module-map.json" | {export @id=html @out="dist/page.html" @content-type="text/html"}}}}"#;
        let config_uri = fixture.write("graph.cem", graph_bytes);
        fixture.write(
            "src/page.html",
            br#"<script type="importmap">{"imports":{"@pkg/runtime":"../node_modules/@pkg/runtime.js"}}</script>"#,
        );
        fixture.write(
            "maps/source.module-map.json",
            br#"{
  "$schema": "https://cem.dev/ns/data/module-map/2",
  "imports": {
    "@pkg/runtime": "../node_modules/@pkg/runtime.js"
  },
  "resources": {
    "@pkg/runtime/sidecar": {"path": "../node_modules/@pkg/sidecar.js", "contentType": "text/javascript"},
    "@pkg/runtime/worker": {"path": "../node_modules/@pkg/worker.mjs", "contentType": "text/javascript"},
    "@pkg/runtime/styles": {"path": "../node_modules/@pkg/styles.css", "contentType": "text/css"},
    "@pkg/runtime/wasm": {"path": "../node_modules/@pkg/runtime.wasm", "contentType": "application/wasm"}
  }
}"#,
        );
        fixture.write(
            "maps/dist.module-map.json",
            br#"{
  "$schema": "https://cem.dev/ns/data/module-map/2",
  "imports": {
    "@pkg/runtime": "./vendor/runtime.js"
  },
  "resources": {
    "@pkg/runtime/sidecar": {"path": "./vendor/sidecar.js", "contentType": "text/javascript"},
    "@pkg/runtime/worker": {"path": "./vendor/worker.mjs", "contentType": "text/javascript"},
    "@pkg/runtime/styles": {"path": "./vendor/styles.css", "contentType": "text/css"},
    "@pkg/runtime/wasm": {"path": "./vendor/runtime.wasm", "contentType": "application/wasm"}
  }
}"#,
        );
        fixture.write(
            "node_modules/@pkg/runtime.js",
            b"export const runtime = 'declared';\n",
        );
        fixture.write(
            "node_modules/@pkg/sidecar.js",
            b"export const sidecar = true;\n",
        );
        fixture.write(
            "node_modules/@pkg/worker.mjs",
            b"self.postMessage('ready');\n",
        );
        fixture.write(
            "node_modules/@pkg/styles.css",
            b".runtime { color: red; }\n",
        );
        fixture.write(
            "node_modules/@pkg/runtime.wasm",
            &[0x00, 0x61, 0x73, 0x6d, 0xff, 0x00],
        );
        fixture.write("node_modules/@pkg/undeclared.js", b"throw new Error();\n");
        let graph = graph(&config_uri, graph_bytes);
        let context = EngineContext::default();
        let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);

        let request = lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
            .expect("dedicated module-map v2 lowers");

        assert_eq!(request.imports.len(), 6);
        assert_eq!(request.exports.len(), 6);
        assert_eq!(request.module_asset_manifest.asset_count, 5);
        assert_eq!(
            request.importmap_rewrites[0].target_imports,
            BTreeMap::from([("@pkg/runtime".to_owned(), "./vendor/runtime.js".to_owned())])
        );
        let assets = request
            .module_asset_manifest
            .assets
            .iter()
            .map(|asset| {
                (
                    asset.specifier.as_str(),
                    asset.target.as_str(),
                    asset.content_type.as_str(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert!(assets.contains(&(
            "@pkg/runtime/sidecar",
            "./vendor/sidecar.js",
            "text/javascript"
        )));
        assert!(assets.contains(&(
            "@pkg/runtime/worker",
            "./vendor/worker.mjs",
            "text/javascript"
        )));
        assert!(assets.contains(&("@pkg/runtime/styles", "./vendor/styles.css", "text/css")));
        assert!(assets.contains(&(
            "@pkg/runtime/wasm",
            "./vendor/runtime.wasm",
            "application/wasm"
        )));
        assert!(request
            .imports
            .iter()
            .all(|import| !import.input.uri.ends_with("undeclared.js")));
    }

    #[test]
    fn dedicated_module_map_v3_rewrites_only_declared_module_edges() {
        let fixture = FixtureDir::new();
        let graph_bytes = br#"{run | {import @id=page @src="src/page.html" @content-type="text/html" | {rewrite-importmap @id=modules @source-map="maps/source.module-map.json" @target-map="maps/dist.module-map.json" | {export @id=html @out="dist/page.html" @content-type="text/html"}}}}"#;
        let config_uri = fixture.write("graph.cem", graph_bytes);
        fixture.write(
            "src/page.html",
            br#"<script type="importmap">{"imports":{"@pkg/app":"../runtime/app.js","@pkg/metadata":"../runtime/runtime.json","@pkg/runtime":"../runtime/runtime.js"}}</script>"#,
        );
        fixture.write(
            "maps/source.module-map.json",
            br#"{
  "$schema": "https://cem.dev/ns/data/module-map/3",
  "imports": {
    "@pkg/app": {"path":"../runtime/app.js","contentType":"text/javascript","moduleImports":{"@pkg/metadata":"@pkg/metadata","@pkg/runtime":"@pkg/runtime"}},
    "@pkg/metadata": {"path":"../runtime/runtime.json","contentType":"application/json"},
    "@pkg/runtime": {"path":"../runtime/runtime.js","contentType":"text/javascript"}
  },
  "resources": {
    "@pkg/worker": {"path":"../runtime/worker.js","contentType":"text/javascript","moduleImports":{"@pkg/runtime":"@pkg/runtime"}}
  }
}"#,
        );
        fixture.write(
            "maps/dist.module-map.json",
            br#"{
  "$schema": "https://cem.dev/ns/data/module-map/3",
  "imports": {
    "@pkg/app": {"path":"./assets/app/app.js","contentType":"text/javascript","moduleImports":{"@pkg/metadata":"@pkg/metadata","@pkg/runtime":"@pkg/runtime"}},
    "@pkg/metadata": {"path":"./assets/runtime/runtime.json","contentType":"application/json"},
    "@pkg/runtime": {"path":"./assets/runtime/runtime.js","contentType":"text/javascript"}
  },
  "resources": {
    "@pkg/worker": {"path":"./workers/worker.js","contentType":"text/javascript","moduleImports":{"@pkg/runtime":"@pkg/runtime"}}
  }
}"#,
        );
        let app_source = br#"const quoted = "import '@pkg/ignored'";
/* export { fake } from '@pkg/ignored'; */
const expression = /import '@pkg\/ignored'/;
import metadata from '@pkg/metadata' with { type: 'json' };
export { runtime } from '@pkg/runtime';
export const loadRuntime = () => import('@pkg/runtime');
export { metadata };
"#;
        let app_uri = fixture.write("runtime/app.js", app_source);
        fixture.write("runtime/runtime.json", br#"{"abi":"v1"}"#);
        fixture.write("runtime/runtime.js", b"export const runtime = true;\n");
        let worker_uri = fixture.write(
            "runtime/worker.js",
            b"import { runtime } from '@pkg/runtime';\nself.postMessage(runtime);\n",
        );
        let graph = graph(&config_uri, graph_bytes);
        let context = EngineContext::default();
        let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);

        let request = lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
            .expect("dedicated module-map v3 lowers");

        let app = request
            .imports
            .iter()
            .find(|import| import.input.uri == app_uri)
            .expect("rewritten app import");
        let app_output = std::str::from_utf8(&app.input.bytes).expect("rewritten app UTF-8");
        assert!(app_output.contains("from '../runtime/runtime.json'"));
        assert!(app_output.contains("from '../runtime/runtime.js'"));
        assert!(app_output.contains("import('../runtime/runtime.js')"));
        assert!(app_output.contains("import '@pkg/ignored'"));
        assert!(app_output.contains("from '@pkg/ignored'"));

        let worker = request
            .imports
            .iter()
            .find(|import| import.input.uri == worker_uri)
            .expect("rewritten worker import");
        assert_eq!(
            std::str::from_utf8(&worker.input.bytes).unwrap(),
            "import { runtime } from '../assets/runtime/runtime.js';\nself.postMessage(runtime);\n"
        );
        assert_eq!(
            request.importmap_rewrites[0].target_imports["@pkg/metadata"],
            "./assets/runtime/runtime.json"
        );
        let app_asset = request
            .module_asset_manifest
            .assets
            .iter()
            .find(|asset| asset.specifier == "@pkg/app")
            .expect("app manifest record");
        assert_eq!(app_asset.source_byte_length, app_source.len() as u64);
        assert_eq!(
            app_asset.source_sha256,
            crate::command_service::sha256_hex(app_source)
        );
        assert_ne!(app_asset.source_sha256, app_asset.sha256);
        assert_eq!(request.module_asset_manifest.contract_version, 2);
    }

    #[test]
    fn module_map_v3_rejects_undeclared_unused_and_mismatched_edges() {
        fn lowering_error(source: &str, target: &str, javascript: &[u8]) -> String {
            let fixture = FixtureDir::new();
            let graph_bytes = br#"{run | {import @id=page @src="src/page.html" @content-type="text/html" | {rewrite-importmap @id=modules @source-map="maps/source.module-map.json" @target-map="maps/dist.module-map.json" | {export @id=html @out="dist/page.html" @content-type="text/html"}}}}"#;
            let config_uri = fixture.write("graph.cem", graph_bytes);
            fixture.write("src/page.html", b"<html></html>");
            fixture.write("maps/source.module-map.json", source.as_bytes());
            fixture.write("maps/dist.module-map.json", target.as_bytes());
            fixture.write("runtime/app.js", javascript);
            fixture.write("runtime/runtime.js", b"export const runtime = true;\n");
            let graph = graph(&config_uri, graph_bytes);
            let context = EngineContext::default();
            let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);
            lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
                .unwrap_err()
                .code()
                .to_owned()
        }

        let source_without_edges = r#"{"$schema":"https://cem.dev/ns/data/module-map/3","imports":{"@pkg/app":{"path":"../runtime/app.js","contentType":"text/javascript"},"@pkg/runtime":{"path":"../runtime/runtime.js","contentType":"text/javascript"}}}"#;
        let target_without_edges = r#"{"$schema":"https://cem.dev/ns/data/module-map/3","imports":{"@pkg/app":{"path":"./app.js","contentType":"text/javascript"},"@pkg/runtime":{"path":"./runtime.js","contentType":"text/javascript"}}}"#;
        assert_eq!(
            lowering_error(
                source_without_edges,
                target_without_edges,
                b"import '@pkg/runtime';\n"
            ),
            "cem.module_map.javascript_bare_specifier_undeclared"
        );

        let source_with_edge = r#"{"$schema":"https://cem.dev/ns/data/module-map/3","imports":{"@pkg/app":{"path":"../runtime/app.js","contentType":"text/javascript","moduleImports":{"@pkg/runtime":"@pkg/runtime"}},"@pkg/runtime":{"path":"../runtime/runtime.js","contentType":"text/javascript"}}}"#;
        let target_with_edge = r#"{"$schema":"https://cem.dev/ns/data/module-map/3","imports":{"@pkg/app":{"path":"./app.js","contentType":"text/javascript","moduleImports":{"@pkg/runtime":"@pkg/runtime"}},"@pkg/runtime":{"path":"./runtime.js","contentType":"text/javascript"}}}"#;
        assert_eq!(
            lowering_error(source_with_edge, target_with_edge, b"export const app = true;\n"),
            "cem.module_map.javascript_rewrite_unused"
        );

        let target_mismatch = target_with_edge.replace("@pkg/runtime\"}", "@pkg/app\"}");
        assert_eq!(
            lowering_error(source_with_edge, &target_mismatch, b"import '@pkg/runtime';\n"),
            "cem.module_map.module_imports_mismatch"
        );
    }

    #[test]
    fn module_map_v2_rejects_invalid_resource_contracts() {
        fn lowering_error(source_map: &str, target_map: &str) -> String {
            let fixture = FixtureDir::new();
            let graph_bytes = br#"{run | {import @id=page @src="src/page.html" @content-type="text/html" | {rewrite-importmap @id=modules @source-map="maps/source.module-map.json" @target-map="maps/dist.module-map.json" | {export @id=html @out="dist/page.html" @content-type="text/html"}}}}"#;
            let config_uri = fixture.write("graph.cem", graph_bytes);
            fixture.write("src/page.html", b"<html></html>");
            fixture.write("maps/source.module-map.json", source_map.as_bytes());
            fixture.write("maps/dist.module-map.json", target_map.as_bytes());
            let graph = graph(&config_uri, graph_bytes);
            let context = EngineContext::default();
            let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);

            lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
                .unwrap_err()
                .code()
                .to_owned()
        }

        let schema = MODULE_MAP_V2_SCHEMA_URI;
        assert_eq!(
            lowering_error(
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{}},"resources":{{"@pkg/styles":{{"path":"../styles.css","contentType":"text/css"}}}}}}"#
                ),
                &format!(r#"{{"$schema":"{schema}","imports":{{}},"resources":{{}}}}"#),
            ),
            "cem.module_map.resource_entries_mismatch"
        );
        assert_eq!(
            lowering_error(
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{"@pkg/runtime":"../runtime.js"}},"resources":{{"@pkg/runtime":{{"path":"../runtime.wasm","contentType":"application/wasm"}}}}}}"#
                ),
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{"@pkg/runtime":"./runtime.js"}},"resources":{{"@pkg/runtime":{{"path":"./runtime.wasm","contentType":"application/wasm"}}}}}}"#
                ),
            ),
            "cem.module_map.asset_identity_duplicate"
        );
        assert_eq!(
            lowering_error(
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{}},"resources":{{"@pkg/styles":{{"path":"../styles.css","contentType":"text/css"}}}}}}"#
                ),
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{}},"resources":{{"@pkg/styles":{{"path":"./styles.js","contentType":"text/javascript"}}}}}}"#
                ),
            ),
            "cem.module_map.resource_type_mismatch"
        );
        assert_eq!(
            lowering_error(
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{}},"resources":{{"@pkg/styles":{{"path":"../styles.css","contentType":"text/css"}}}}}}"#
                ),
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{}},"resources":{{"@pkg/styles":{{"path":"../styles.css","contentType":"text/css"}}}}}}"#
                ),
            ),
            "cem.module_map.resource_target_invalid"
        );
        assert_eq!(
            lowering_error(
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{}},"resources":{{"@pkg/image":{{"path":"../image.png","contentType":"image/png"}}}}}}"#
                ),
                &format!(
                    r#"{{"$schema":"{schema}","imports":{{}},"resources":{{"@pkg/image":{{"path":"./image.png","contentType":"image/png"}}}}}}"#
                ),
            ),
            "cem.module_map.resource_type_unsupported"
        );
    }

    #[test]
    fn module_map_contract_rejects_discovery_and_output_escape_shapes() {
        for specifier in ["./runtime.js", "https://cdn.example/runtime.js", "@pkg/"] {
            let error = validate_module_specifier("graph.cem", specifier).unwrap_err();
            assert_eq!(error.code(), "cem.module_map.specifier_invalid");
        }
        for target in [
            "../runtime.js",
            "./../runtime.js",
            "./runtime.js?x=1",
        ] {
            let error = validate_module_import_target(
                "graph.cem",
                "@pkg/runtime",
                target,
                "text/javascript",
            )
            .unwrap_err();
            assert_eq!(error.code(), "cem.module_map.target_invalid");
        }
        let error = validate_module_import_target(
            "graph.cem",
            "@pkg/runtime",
            "./runtime.css",
            "text/javascript",
        )
        .unwrap_err();
        assert_eq!(error.code(), "cem.module_map.import_type_mismatch");
    }

    #[test]
    fn dedicated_module_maps_require_identical_keys() {
        let fixture = FixtureDir::new();
        let graph_bytes = br#"{run | {import @id=page @src="src/page.html" @content-type="text/html" | {rewrite-importmap @id=modules @source-map="maps/source.module-map.json" @target-map="maps/dist.module-map.json" | {export @id=html @out="dist/page.html" @content-type="text/html"}}}}"#;
        let config_uri = fixture.write("graph.cem", graph_bytes);
        fixture.write("src/page.html", b"<html></html>");
        fixture.write(
            "maps/source.module-map.json",
            br#"{"$schema":"https://cem.dev/ns/data/module-map/1","imports":{"@pkg/runtime":"../node_modules/@pkg/runtime.js"}}"#,
        );
        fixture.write(
            "maps/dist.module-map.json",
            br#"{"$schema":"https://cem.dev/ns/data/module-map/1","imports":{"@pkg/other":"./vendor/other.js"}}"#,
        );
        let graph = graph(&config_uri, graph_bytes);
        let context = EngineContext::default();
        let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);

        let error = lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
            .unwrap_err();

        assert_eq!(error.code(), "cem.module_map.entries_mismatch");
    }

    #[test]
    fn dedicated_module_map_destination_collisions_are_runtime_contract_errors() {
        let fixture = FixtureDir::new();
        let graph_bytes = br#"{run | {import @id=page @src="src/page.html" @content-type="text/html" | {rewrite-importmap @id=modules @source-map="maps/source.module-map.json" @target-map="maps/dist.module-map.json" @missing=insert | {export @id=html @out="dist/page.html" @content-type="text/html"}}}}"#;
        let config_uri = fixture.write("graph.cem", graph_bytes);
        fixture.write("src/page.html", b"<html><head></head></html>");
        fixture.write(
            "maps/source.module-map.json",
            br#"{"$schema":"https://cem.dev/ns/data/module-map/1","imports":{"@pkg/a":"../node_modules/@pkg/a.js","@pkg/b":"../node_modules/@pkg/b.js"}}"#,
        );
        fixture.write(
            "maps/dist.module-map.json",
            br#"{"$schema":"https://cem.dev/ns/data/module-map/1","imports":{"@pkg/a":"./vendor/shared.js","@pkg/b":"./vendor/shared.js"}}"#,
        );
        fixture.write("node_modules/@pkg/a.js", b"export const a = 1;\n");
        fixture.write("node_modules/@pkg/b.js", b"export const b = 2;\n");
        let graph = graph(&config_uri, graph_bytes);
        let context = EngineContext::default();
        let provider = FilesystemTransformGraphResourceProvider::new(&context, &config_uri);
        let request = lower_transform_graph_request(&context, &graph, &provider, &config_uri, true)
            .expect("colliding destinations lower before runtime validation");

        let diagnostics = engine::validate_transform_graph_runtime_contract(&request);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.transform_runtime.duplicate_destination"
        }));
    }
}
