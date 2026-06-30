//! CLI dispatch layer.
//!
//! Translates `cli` args into engine requests, calls the engine, serializes the
//! response, and applies exit-code policy from `cem-ml-cli-contract.md`.

#![allow(clippy::items_after_test_module)]

use crate::cli;
use crate::template_pass;
use cem_ml::engine::{self as eng, CemMlEngine, EngineError};
use cem_ml::resolver::{
    is_windows_drive_path, local_file_uri_to_path, local_path_or_file_uri, uri_scheme,
    ResolveDirection, ResolveListRequest, ResolvePurpose, ResolveRequest, ResolvedListEntry,
    ResolvedRead, ResolvedWrite, ResolverDiagnostic, ResolverRegistry, ResourceResolver,
};
use cem_ml::run_config::{
    self, InputSpec, OutputSpec, ResolverSpec, RunConfig, RunConfigDefaults, ScopeConfig,
};
use cem_ml::transform_config::{
    self, TransformGraphConfig, TransformGraphEdgeRole, TransformGraphJoinMode, TransformGraphNode,
    TransformGraphNodeKind,
};
use cem_ml_transform_cem_ql::register_cem_ql_template_adapter;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const EXIT_OK: u8 = 0;
pub const EXIT_HARD_FAILURE: u8 = 1;
pub const EXIT_USAGE_OR_RESERVED: u8 = 2;
pub const EXIT_SCHEMA: u8 = 3;
pub const EXIT_IO: u8 = 6;
pub const EXIT_INTERNAL: u8 = 7;

pub struct Outcome {
    pub exit_code: u8,
}

impl Outcome {
    pub fn ok() -> Self {
        Self { exit_code: EXIT_OK }
    }
    pub fn code(c: u8) -> Self {
        Self { exit_code: c }
    }
}

pub struct Streams<'a> {
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
    pub quiet: bool,
}

enum CliRequestError {
    Usage(String),
    RunConfigDiagnostics {
        config: Option<Box<RunConfig>>,
        diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
    },
    Engine(EngineError),
}

const READ_PURPOSES: [ResolvePurpose; 4] = [
    ResolvePurpose::Config,
    ResolvePurpose::Input,
    ResolvePurpose::Template,
    ResolvePurpose::ModuleMap,
];

const WRITE_PURPOSES: [ResolvePurpose; 3] = [
    ResolvePurpose::Output,
    ResolvePurpose::Report,
    ResolvePurpose::ObserveEvents,
];

const TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES: usize = 1024;

#[derive(Debug, Clone)]
struct LocalMirrorMapping {
    uri_prefix: String,
    local_root: PathBuf,
}

#[derive(Debug, Clone)]
struct LocalMirrorResolver {
    mappings: Vec<LocalMirrorMapping>,
}

impl LocalMirrorResolver {
    fn new(mappings: Vec<LocalMirrorMapping>) -> Self {
        let mut mappings = mappings;
        mappings.sort_by_key(|mapping| std::cmp::Reverse(mapping.uri_prefix.len()));
        Self { mappings }
    }

    fn schemes(&self) -> BTreeSet<String> {
        self.mappings
            .iter()
            .filter_map(|mapping| uri_scheme(&mapping.uri_prefix).map(str::to_owned))
            .collect()
    }

    fn local_path(&self, request: &ResolveRequest) -> Result<PathBuf, ResolverDiagnostic> {
        let (mapping, suffix) =
            self.mapping_for(&request.uri, request.purpose, request.direction)?;
        local_mirror_path(&mapping.local_root, &suffix).map_err(|message| ResolverDiagnostic::Io {
            uri: request.uri.clone(),
            message,
        })
    }

    fn mapping_for(
        &self,
        uri: &str,
        purpose: ResolvePurpose,
        direction: ResolveDirection,
    ) -> Result<(&LocalMirrorMapping, String), ResolverDiagnostic> {
        let Some(mapping) = self
            .mappings
            .iter()
            .find(|mapping| uri.starts_with(&mapping.uri_prefix))
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: uri.to_owned(),
                purpose,
                direction,
            });
        };
        Ok((
            mapping,
            uri.strip_prefix(&mapping.uri_prefix)
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_owned(),
        ))
    }
}

impl ResourceResolver for LocalMirrorResolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let path = self.local_path(request)?;
        fs::read(&path)
            .map(|bytes| ResolvedRead {
                uri: request.uri.clone(),
                bytes,
                content_type: request.content_type_hint.clone(),
            })
            .map_err(|error| ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: error.to_string(),
            })
    }

    fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        let path = self.local_path(request)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: error.to_string(),
            })?;
        }
        fs::write(&path, bytes).map_err(|error| ResolverDiagnostic::Io {
            uri: request.uri.clone(),
            message: error.to_string(),
        })?;
        Ok(ResolvedWrite {
            uri: request.uri.clone(),
        })
    }

    fn list(
        &self,
        request: &ResolveListRequest,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        let (mapping, suffix) =
            self.mapping_for(&request.uri, request.purpose, ResolveDirection::List)?;
        let pattern_path = local_mirror_path(&mapping.local_root, &suffix).map_err(|message| {
            ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message,
            }
        })?;
        let pattern_file = pattern_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (prefix, suffix_match) = pattern_file.split_once('*').unwrap_or(("", ""));
        let parent = transform_graph_glob_parent(&pattern_path);
        let paths = transform_graph_collect_import_glob_matches(&parent, prefix, suffix_match)
            .map_err(|error| ResolverDiagnostic::Io {
                uri: request.uri.clone(),
                message: error.to_string(),
            })?;
        let mut entries = Vec::new();
        for path in paths {
            let relative =
                path.strip_prefix(&mapping.local_root)
                    .map_err(|error| ResolverDiagnostic::Io {
                        uri: request.uri.clone(),
                        message: error.to_string(),
                    })?;
            entries.push(ResolvedListEntry {
                uri: local_mirror_uri(&mapping.uri_prefix, relative),
                content_type: request.content_type_hint.clone(),
            });
        }
        entries.sort_by(|left, right| left.uri.cmp(&right.uri));
        if let Some(max_entries) = request.max_entries {
            if entries.len() > max_entries {
                entries.truncate(max_entries + 1);
            }
        }
        Ok(entries)
    }
}

fn local_mirror_path(root: &Path, suffix: &str) -> Result<PathBuf, String> {
    if suffix.contains('?') || suffix.contains('#') || suffix.contains('\\') {
        return Err("resolver URI suffix contains unsupported path characters".to_owned());
    }
    let mut path = root.to_path_buf();
    for segment in suffix.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err("resolver URI suffix must not escape the local root".to_owned());
        }
        path.push(segment);
    }
    Ok(path)
}

fn local_mirror_uri(prefix: &str, relative: &Path) -> String {
    let relative = path_display_slash(relative);
    if relative.is_empty() {
        return prefix.to_owned();
    }
    if prefix.ends_with('/') {
        format!("{prefix}{relative}")
    } else {
        format!("{prefix}/{relative}")
    }
}

fn read_source(
    context: &eng::EngineContext,
    path: &Path,
    label: &str,
    purpose: ResolvePurpose,
    content_type_hint: Option<&str>,
) -> io::Result<ResolvedRead> {
    let raw = path.to_string_lossy();
    if raw.starts_with("file://") {
        if let Some(path) = local_file_uri_to_path(&raw) {
            return Ok(ResolvedRead {
                uri: raw.into_owned(),
                bytes: fs::read(path)?,
                content_type: None,
            });
        }
        if let Some(read) = read_registered_source(context, &raw, purpose, content_type_hint)? {
            return Ok(read);
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported {label} `{raw}`; only local file:// URIs are supported"),
        ));
    }

    if uri_scheme(&raw).is_some() && !is_windows_drive_path(&raw) {
        if let Some(read) = read_registered_source(context, &raw, purpose, content_type_hint)? {
            return Ok(read);
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported {label} `{raw}`; remote/custom URI resolvers are not implemented"),
        ));
    }

    Ok(ResolvedRead {
        uri: raw.into_owned(),
        bytes: fs::read(path)?,
        content_type: None,
    })
}

fn read_registered_source(
    context: &eng::EngineContext,
    uri: &str,
    purpose: ResolvePurpose,
    content_type_hint: Option<&str>,
) -> io::Result<Option<ResolvedRead>> {
    let mut request = ResolveRequest::new(uri, purpose, ResolveDirection::Read);
    if let Some(content_type_hint) = content_type_hint {
        request = request.with_content_type_hint(content_type_hint);
    }
    match context.resolver_registry.read(&request) {
        Ok(read) => Ok(Some(read)),
        Err(ResolverDiagnostic::UnsupportedResolver { .. }) => Ok(None),
        Err(error) => Err(io::Error::other(error)),
    }
}

fn engine_input(
    context: &eng::EngineContext,
    path: &Path,
    from_format: Option<cli::InputFormat>,
    root_scope: ScopeConfig,
) -> Result<eng::EngineInput, EngineError> {
    let mut root_scope = root_scope;
    let read = read_source(
        context,
        path,
        "input URI",
        ResolvePurpose::Input,
        root_scope.default_content_type.as_deref(),
    )
    .map_err(|e| EngineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    if root_scope.default_content_type.is_none() {
        root_scope.default_content_type = read.content_type.clone();
    }
    apply_schema_registry_defaults(context, &mut root_scope);
    Ok(eng::EngineInput {
        uri: path.display().to_string(),
        bytes: read.bytes,
        from_format: from_format.map(to_engine_input_format),
        identity: root_scope.format_identity_option(),
        root_scope,
    })
}

fn positional_input_scope(path: &Path, defaults: &ScopeConfig) -> ScopeConfig {
    let mut scope = defaults.clone();
    if scope.default_content_type.is_none() {
        scope.default_content_type =
            run_config::infer_content_type_from_path(&path.display().to_string());
    }
    scope
}

fn placeholder_input(
    path: &Path,
    from_format: Option<cli::InputFormat>,
    root_scope: ScopeConfig,
) -> eng::EngineInput {
    let mut root_scope = root_scope;
    let context = eng::EngineContext::default();
    apply_schema_registry_defaults(&context, &mut root_scope);
    eng::EngineInput {
        uri: path.display().to_string(),
        bytes: Vec::new(),
        from_format: from_format.map(to_engine_input_format),
        identity: root_scope.format_identity_option(),
        root_scope,
    }
}

fn engine_input_from_spec(
    context: &eng::EngineContext,
    spec: &InputSpec,
    from_format: Option<cli::InputFormat>,
) -> Result<eng::EngineInput, CliRequestError> {
    let path = Path::new(&spec.uri);
    let mut root_scope = spec.root_scope.clone();
    let read = read_source(
        context,
        path,
        "input URI",
        ResolvePurpose::Input,
        root_scope.default_content_type.as_deref(),
    )
    .map_err(|source| {
        CliRequestError::Engine(EngineError::Io {
            path: path.into(),
            source,
        })
    })?;
    if root_scope.default_content_type.is_none() {
        root_scope.default_content_type = read.content_type.clone();
    }
    apply_schema_registry_defaults(context, &mut root_scope);
    Ok(eng::EngineInput {
        uri: spec.uri.clone(),
        bytes: read.bytes,
        from_format: from_format.map(to_engine_input_format),
        identity: root_scope.format_identity_option(),
        root_scope,
    })
}

fn template_input(
    context: &eng::EngineContext,
    path: &Path,
    root_scope: ScopeConfig,
) -> Result<eng::TemplateInput, EngineError> {
    let mut root_scope = root_scope;
    let read = read_source(
        context,
        path,
        "template URI",
        ResolvePurpose::Template,
        root_scope.default_content_type.as_deref(),
    )
    .map_err(|e| EngineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    if root_scope.default_content_type.is_none() {
        root_scope.default_content_type = read.content_type.clone();
    }
    apply_schema_registry_defaults(context, &mut root_scope);
    Ok(eng::TemplateInput {
        uri: path.display().to_string(),
        bytes: read.bytes,
        identity: root_scope.format_identity_option(),
        root_scope,
    })
}

fn apply_schema_registry_defaults(context: &eng::EngineContext, scope: &mut ScopeConfig) {
    if scope.schema.is_some() {
        return;
    }
    let Some(content_type) = scope.default_content_type.as_deref() else {
        return;
    };
    if let Ok(descriptor) = context.schema_registry.resolve_content_type(content_type) {
        scope.schema = Some(descriptor.schema_uri.clone());
    }
}

fn run_config_with_context(
    context: &eng::EngineContext,
    options: &cli::RunOptions,
    defaults: RunConfigDefaults,
) -> Result<RunConfig, CliRequestError> {
    let mut config_base_uri = None;
    let mut config = if let Some(path) = &options.config {
        let local_config_path = local_path_or_file_uri(path, "config path").ok();
        let config_source_uri = path.display().to_string();
        let read = read_source(
            context,
            path,
            "config path",
            ResolvePurpose::Config,
            options.config_content_type.as_deref(),
        )
        .map_err(|source| {
            CliRequestError::Engine(EngineError::Io {
                path: path.clone(),
                source,
            })
        })?;
        config_base_uri = Some(
            local_config_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| read.uri.clone()),
        );
        let identity = eng::FormatIdentity {
            content_type: options
                .config_content_type
                .clone()
                .or_else(|| {
                    local_config_path
                        .as_ref()
                        .and_then(|path| infer_config_content_type(path.as_ref()))
                })
                .or_else(|| infer_config_content_type(path))
                .or_else(|| read.content_type.clone()),
            schema: Some(
                options
                    .config_schema
                    .clone()
                    .unwrap_or_else(|| run_config::RUN_CONFIG_SCHEMA_URI.to_owned()),
            ),
            default_namespace: None,
            namespaces: BTreeMap::new(),
            base_uri: Some(config_source_uri.clone()),
        };
        run_config::parse_run_config(run_config::RunConfigParseRequest {
            bytes: read.bytes,
            identity,
            base_uri: Some(config_source_uri.clone()),
        })
        .map_err(|error| CliRequestError::RunConfigDiagnostics {
            config: None,
            diagnostics: vec![run_config_error_diagnostic(
                error.code,
                error.message,
                Some(config_source_uri),
            )],
        })
        .map(|response| response.config)?
    } else {
        RunConfig::default()
    };

    for record in &options.input_specs {
        let spec = run_config::parse_input_spec_record(record)
            .map_err(|error| CliRequestError::Usage(format!("invalid --input-spec: {error}")))?;
        config.inputs.push(spec);
    }
    for record in &options.output_specs {
        let spec = run_config::parse_output_spec_record(record)
            .map_err(|error| CliRequestError::Usage(format!("invalid --output-spec: {error}")))?;
        config.outputs.push(spec);
    }

    let response = run_config::normalize_run_config(config, defaults, config_base_uri.as_deref());
    if response.diagnostics.is_empty() {
        Ok(response.config)
    } else {
        Err(CliRequestError::RunConfigDiagnostics {
            config: Some(Box::new(response.config)),
            diagnostics: response.diagnostics,
        })
    }
}

fn run_config_error_diagnostic(
    code: &str,
    message: String,
    uri: Option<String>,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri,
        code: code.to_owned(),
        severity: cem_ml::diagnostics::Severity::Fatal,
        message,
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn infer_config_content_type(path: &Path) -> Option<String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => Some("application/json".to_owned()),
        Some("yaml") | Some("yml") => Some("application/yaml".to_owned()),
        Some("cem") => Some("application/cem+xml".to_owned()),
        Some("csv") => Some("text/csv".to_owned()),
        _ => None,
    }
}

fn collect_configured_inputs(
    context: &eng::EngineContext,
    paths: &[PathBuf],
    from_format: Option<cli::InputFormat>,
    config: &RunConfig,
    positional_defaults: &ScopeConfig,
) -> Result<Vec<eng::EngineInput>, CliRequestError> {
    let mut inputs = collect_inputs(context, paths, from_format, positional_defaults)
        .map_err(CliRequestError::Engine)?;
    for spec in &config.inputs {
        inputs.push(engine_input_from_spec(context, spec, from_format)?);
    }
    Ok(inputs)
}

fn single_configured_input(
    context: &eng::EngineContext,
    path: Option<&Path>,
    from_format: Option<cli::InputFormat>,
    config: &RunConfig,
    positional_defaults: &ScopeConfig,
) -> Result<eng::EngineInput, CliRequestError> {
    let mut inputs = Vec::new();
    if let Some(path) = path {
        inputs.push(
            engine_input(
                context,
                path,
                from_format,
                positional_input_scope(path, positional_defaults),
            )
            .map_err(CliRequestError::Engine)?,
        );
    }
    for spec in &config.inputs {
        inputs.push(engine_input_from_spec(context, spec, from_format)?);
    }

    match inputs.len() {
        1 => Ok(inputs.remove(0)),
        0 => Err(CliRequestError::Usage(
            "expected one input path or --input-spec record".to_owned(),
        )),
        _ => Err(CliRequestError::Usage(
            "expected exactly one input for this command; use validate/check/bench for multi-input runs"
                .to_owned(),
        )),
    }
}

fn collect_inputs(
    context: &eng::EngineContext,
    paths: &[std::path::PathBuf],
    from_format: Option<cli::InputFormat>,
    positional_defaults: &ScopeConfig,
) -> Result<Vec<eng::EngineInput>, EngineError> {
    paths
        .iter()
        .map(|p| {
            engine_input(
                context,
                p,
                from_format,
                positional_input_scope(p, positional_defaults),
            )
        })
        .collect()
}

fn collect_fixture_inputs(
    paths: &[PathBuf],
    use_defaults: bool,
    positional_defaults: &ScopeConfig,
) -> Vec<eng::EngineInput> {
    if paths.is_empty() && use_defaults {
        default_fixture_inputs(positional_defaults)
    } else {
        paths
            .iter()
            .map(|p| {
                placeholder_input(
                    p,
                    infer_input_format(p),
                    positional_input_scope(p, positional_defaults),
                )
            })
            .collect()
    }
}

fn infer_input_format(path: &Path) -> Option<cli::InputFormat> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("cem") => Some(cli::InputFormat::Cem),
        Some("html") | Some("htm") => Some(cli::InputFormat::Html),
        Some("xml") => Some(cli::InputFormat::Xml),
        _ => None,
    }
}

const FIXTURE_MANIFEST_JSON: &str = include_str!("../../../examples/cem-ml/fixture-manifest.json");

fn default_fixture_inputs(positional_defaults: &ScopeConfig) -> Vec<eng::EngineInput> {
    fixture_manifest_pairs()
        .into_iter()
        .flat_map(|pair| {
            [
                placeholder_input(
                    Path::new(&pair.cem),
                    Some(cli::InputFormat::Cem),
                    positional_input_scope(Path::new(&pair.cem), positional_defaults),
                ),
                placeholder_input(
                    Path::new(&pair.html),
                    Some(cli::InputFormat::Html),
                    positional_input_scope(Path::new(&pair.html), positional_defaults),
                ),
            ]
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixturePair {
    id: String,
    cem: String,
    html: String,
}

fn fixture_manifest_pairs() -> Vec<FixturePair> {
    let manifest: serde_json::Value =
        serde_json::from_str(FIXTURE_MANIFEST_JSON).expect("fixture manifest JSON must parse");
    let pairs = manifest
        .get("pairs")
        .and_then(|v| v.as_array())
        .expect("fixture manifest must contain a `pairs` array");
    pairs
        .iter()
        .map(|pair| FixturePair {
            id: manifest_string(pair, "id"),
            cem: manifest_string(pair, "cem"),
            html: manifest_string(pair, "html"),
        })
        .collect()
}

fn manifest_string(pair: &serde_json::Value, key: &str) -> String {
    pair.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("fixture manifest pair must contain string `{key}`"))
        .to_owned()
}

fn to_engine_input_format(f: cli::InputFormat) -> eng::InputFormat {
    match f {
        cli::InputFormat::Cem => eng::InputFormat::Cem,
        cli::InputFormat::Html => eng::InputFormat::Html,
        cli::InputFormat::Xml => eng::InputFormat::Xml,
    }
}

fn to_engine_layer_format(f: cli::LayerFormat) -> eng::LayerFormat {
    match f {
        cli::LayerFormat::Cem => eng::LayerFormat::Cem,
        cli::LayerFormat::Html => eng::LayerFormat::Html,
        cli::LayerFormat::Xml => eng::LayerFormat::Xml,
        cli::LayerFormat::DomJson => eng::LayerFormat::DomJson,
        cli::LayerFormat::Ast => eng::LayerFormat::Ast,
        cli::LayerFormat::Events => eng::LayerFormat::Events,
        cli::LayerFormat::DomBin => eng::LayerFormat::DomBin,
        cli::LayerFormat::AstBin => eng::LayerFormat::AstBin,
        cli::LayerFormat::EventsBin => eng::LayerFormat::EventsBin,
    }
}

fn to_engine_parse_projection(f: cli::ParseFormat) -> eng::ParseProjection {
    match f {
        cli::ParseFormat::DomJson => eng::ParseProjection::DomJson,
        cli::ParseFormat::Json => eng::ParseProjection::Json,
        cli::ParseFormat::Ast => eng::ParseProjection::Ast,
        cli::ParseFormat::Events => eng::ParseProjection::Events,
    }
}

fn to_engine_validate_projection(f: cli::ValidateFormat) -> eng::ValidateProjection {
    match f {
        cli::ValidateFormat::Json => eng::ValidateProjection::Json,
        cli::ValidateFormat::Xml => eng::ValidateProjection::Xml,
        cli::ValidateFormat::Cem => eng::ValidateProjection::Cem,
        cli::ValidateFormat::Text => eng::ValidateProjection::Text,
        cli::ValidateFormat::Html => eng::ValidateProjection::Html,
        cli::ValidateFormat::Markdown => eng::ValidateProjection::Markdown,
    }
}

fn to_engine_trace_projection(f: cli::TraceFormat) -> eng::TraceProjection {
    match f {
        cli::TraceFormat::Json => eng::TraceProjection::Json,
        cli::TraceFormat::Xml => eng::TraceProjection::Xml,
        cli::TraceFormat::Cem => eng::TraceProjection::Cem,
        cli::TraceFormat::Text => eng::TraceProjection::Text,
        cli::TraceFormat::Html => eng::TraceProjection::Html,
    }
}

fn to_engine_bench_projection(f: cli::BenchFormat) -> eng::BenchProjection {
    match f {
        cli::BenchFormat::Text => eng::BenchProjection::Text,
        cli::BenchFormat::Json => eng::BenchProjection::Json,
    }
}

fn to_engine_inspect_view(v: cli::InspectView) -> eng::InspectView {
    match v {
        cli::InspectView::Summary => eng::InspectView::Summary,
        cli::InspectView::Ast => eng::InspectView::Ast,
        cli::InspectView::Events => eng::InspectView::Events,
        cli::InspectView::Diagnostics => eng::InspectView::Diagnostics,
        cli::InspectView::SourceOffsets => eng::InspectView::SourceOffsets,
        cli::InspectView::Tree => eng::InspectView::Tree,
    }
}

fn to_engine_fail_level(f: cli::FailLevel) -> eng::FailLevel {
    match f {
        cli::FailLevel::Parse => eng::FailLevel::Parse,
        cli::FailLevel::Validate => eng::FailLevel::Validate,
        cli::FailLevel::Strict => eng::FailLevel::Strict,
    }
}

fn to_engine_bench_profile(p: cli::BenchProfile) -> eng::BenchProfile {
    match p {
        cli::BenchProfile::Cpu => eng::BenchProfile::Cpu,
        cli::BenchProfile::Memory => eng::BenchProfile::Memory,
    }
}

fn context(c: &cli::ContextOptions) -> eng::EngineContext {
    let mut context = eng::EngineContext {
        schema: c.schema.clone(),
        content_type: c.content_type.clone(),
        base_uri: c.base_uri.clone(),
        ..eng::EngineContext::default()
    };
    register_cli_transform_template_adapters(&mut context);
    register_cli_resolvers(&mut context.resolver_registry, c, None);
    context
}

fn context_with_config(c: &cli::ContextOptions, config: &RunConfig) -> eng::EngineContext {
    let mut context = eng::EngineContext {
        scheduler: config.scheduler.clone(),
        ..context(c)
    };
    register_cli_resolvers(&mut context.resolver_registry, c, Some(config));
    context
}

fn register_cli_transform_template_adapters(context: &mut eng::EngineContext) {
    register_cem_ql_template_adapter(&mut context.template_adapter_registry);
}

fn register_cli_resolvers(
    registry: &mut ResolverRegistry,
    c: &cli::ContextOptions,
    config: Option<&RunConfig>,
) {
    let mut read_mappings = c
        .resolver_read_maps
        .iter()
        .map(mapping_from_cli)
        .collect::<Vec<_>>();
    let mut write_mappings = c
        .resolver_write_maps
        .iter()
        .map(mapping_from_cli)
        .collect::<Vec<_>>();
    if let Some(config) = config {
        for spec in &config.resolvers {
            if spec.read {
                read_mappings.push(mapping_from_spec(spec));
            }
            if spec.write {
                write_mappings.push(mapping_from_spec(spec));
            }
        }
    }
    register_resolver_mappings(
        registry,
        read_mappings,
        ResolveDirection::Read,
        &READ_PURPOSES,
    );
    register_resolver_mappings(
        registry,
        write_mappings,
        ResolveDirection::Write,
        &WRITE_PURPOSES,
    );
}

fn register_resolver_mappings(
    registry: &mut ResolverRegistry,
    mappings: Vec<LocalMirrorMapping>,
    direction: ResolveDirection,
    purposes: &[ResolvePurpose],
) {
    if mappings.is_empty() {
        return;
    }
    let resolver = LocalMirrorResolver::new(mappings);
    for scheme in resolver.schemes() {
        for purpose in purposes {
            registry.register(scheme.clone(), *purpose, direction, resolver.clone());
            if direction == ResolveDirection::Read {
                registry.register(
                    scheme.clone(),
                    *purpose,
                    ResolveDirection::List,
                    resolver.clone(),
                );
            }
        }
    }
}

fn mapping_from_cli(mapping: &cli::ResolverMap) -> LocalMirrorMapping {
    LocalMirrorMapping {
        uri_prefix: mapping.uri_prefix.clone(),
        local_root: mapping.local_root.clone(),
    }
}

fn mapping_from_spec(spec: &ResolverSpec) -> LocalMirrorMapping {
    LocalMirrorMapping {
        uri_prefix: spec.uri_prefix.clone(),
        local_root: PathBuf::from(&spec.local_root),
    }
}

fn run_config_for_context(
    context_options: &cli::ContextOptions,
    options: &cli::RunOptions,
    defaults: RunConfigDefaults,
) -> Result<RunConfig, CliRequestError> {
    let engine_context = context(context_options);
    run_config_with_context(&engine_context, options, defaults)
}

fn input_scope_defaults(c: &cli::ContextOptions) -> ScopeConfig {
    ScopeConfig {
        default_content_type: c.content_type.clone(),
        schema: c.schema.clone(),
        default_namespace: c.default_namespace.clone(),
        namespaces: context_namespaces(c),
        module_map: c.module_map.clone(),
        base_uri: c.base_uri.clone(),
        policy: c.scope_policy.clone(),
        version_pins: context_key_values(&c.version_pins),
        budgets: context_key_values(&c.scope_budgets),
    }
}

fn output_scope_defaults(args: &cli::ConvertArgs) -> ScopeConfig {
    ScopeConfig {
        default_content_type: args.to_content_type.clone(),
        schema: args.to_schema.clone(),
        default_namespace: args.context.default_namespace.clone(),
        namespaces: context_namespaces(&args.context),
        module_map: args.context.module_map.clone(),
        base_uri: args.context.base_uri.clone(),
        policy: args.context.scope_policy.clone(),
        version_pins: context_key_values(&args.context.version_pins),
        budgets: context_key_values(&args.context.scope_budgets),
    }
}

fn context_namespaces(c: &cli::ContextOptions) -> BTreeMap<String, String> {
    c.namespaces
        .iter()
        .map(|binding| (binding.prefix.clone(), binding.uri.clone()))
        .collect()
}

fn context_key_values(values: &[cli::ScopeKeyValue]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|entry| (entry.key.clone(), entry.value.clone()))
        .collect()
}

fn run_defaults(input_scope: ScopeConfig, output_scope: ScopeConfig) -> RunConfigDefaults {
    RunConfigDefaults {
        input_scope,
        output_scope,
    }
}

fn convert_target_identity(args: &cli::ConvertArgs) -> Option<eng::FormatIdentity> {
    if args.to_content_type.is_none()
        && args.to_schema.is_none()
        && args.context.default_namespace.is_none()
        && args.context.namespaces.is_empty()
        && args.context.base_uri.is_none()
    {
        return None;
    }
    Some(eng::FormatIdentity {
        content_type: args.to_content_type.clone(),
        schema: args.to_schema.clone(),
        default_namespace: args.context.default_namespace.clone(),
        namespaces: context_namespaces(&args.context),
        base_uri: args.context.base_uri.clone(),
    })
}

fn convert_target_identity_with_config(
    args: &cli::ConvertArgs,
    config: &RunConfig,
) -> Option<eng::FormatIdentity> {
    if let Some(output) = config.outputs.first() {
        let identity = output.root_scope.format_identity();
        if identity.content_type.is_some()
            || identity.schema.is_some()
            || identity.default_namespace.is_some()
            || !identity.namespaces.is_empty()
            || identity.base_uri.is_some()
        {
            return Some(identity);
        }
    }
    convert_target_identity(args)
}

fn convert_target_scope(args: &cli::ConvertArgs) -> ScopeConfig {
    output_scope_defaults(args)
}

fn transform_data_scope(args: &cli::TransformArgs, data: &Path) -> ScopeConfig {
    ScopeConfig {
        default_content_type: args
            .data_content_type
            .clone()
            .or_else(|| run_config::infer_content_type_from_path(&data.display().to_string())),
        schema: args.data_schema.clone(),
        ..ScopeConfig::default()
    }
}

fn transform_template_scope(args: &cli::TransformArgs, template: &Path) -> ScopeConfig {
    ScopeConfig {
        default_content_type: args
            .template_content_type
            .clone()
            .or_else(|| run_config::infer_content_type_from_path(&template.display().to_string())),
        schema: args.template_schema.clone(),
        ..ScopeConfig::default()
    }
}

fn transform_target_scope(args: &cli::TransformArgs) -> ScopeConfig {
    ScopeConfig {
        default_content_type: args.to_content_type.clone(),
        schema: args.to_schema.clone(),
        ..ScopeConfig::default()
    }
}

fn transform_template_entrypoint(value: Option<&str>) -> eng::TransformTemplateEntrypoint {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(eng::TransformTemplateEntrypoint::named)
        .unwrap_or_else(eng::TransformTemplateEntrypoint::implicit)
}

fn transform_parse_cli_params(
    raw_params: &[String],
) -> Result<BTreeMap<String, serde_json::Value>, CliRequestError> {
    let mut params = BTreeMap::new();
    for raw in raw_params {
        let Some((name, value)) = raw.split_once('=') else {
            return Err(CliRequestError::Usage(format!(
                "transform --param `{raw}` must use NAME=VALUE"
            )));
        };
        let name = name.trim();
        if name.is_empty() {
            return Err(CliRequestError::Usage(
                "transform --param name must not be empty".into(),
            ));
        }
        if params
            .insert(name.to_owned(), serde_json::Value::String(value.to_owned()))
            .is_some()
        {
            return Err(CliRequestError::Usage(format!(
                "transform --param `{name}` is declared more than once"
            )));
        }
    }
    Ok(params)
}

fn transform_execution_policy_for(
    template_kind: eng::TransformTemplateKind,
    entrypoint: &eng::TransformTemplateEntrypoint,
    params: &BTreeMap<String, serde_json::Value>,
) -> eng::TransformExecutionPolicy {
    let mut policy = eng::TransformExecutionPolicy::default();
    if template_kind == eng::TransformTemplateKind::Xslt {
        policy.runtime_phase = eng::TransformRuntimePhase::XsltParity;
    } else if !entrypoint.is_implicit() || !params.is_empty() {
        policy.runtime_phase = eng::TransformRuntimePhase::CemNativeModules;
    }
    policy
}

fn validate_transform_template_module_surface(
    template_kind: eng::TransformTemplateKind,
    entrypoint: &eng::TransformTemplateEntrypoint,
    params: &BTreeMap<String, serde_json::Value>,
) -> Result<(), CliRequestError> {
    if !matches!(
        template_kind,
        eng::TransformTemplateKind::CemNative | eng::TransformTemplateKind::Xslt
    ) && (!entrypoint.is_implicit() || !params.is_empty())
    {
        return Err(CliRequestError::Usage(
            "transform template entrypoints and params are supported only for executable template adapters"
                .into(),
        ));
    }
    Ok(())
}

fn transform_request_from_args(
    context: &eng::EngineContext,
    args: &cli::TransformArgs,
) -> Result<eng::TransformRequest, CliRequestError> {
    let data_path = args
        .data
        .as_deref()
        .ok_or_else(|| CliRequestError::Usage("transform requires DATA or --config".into()))?;
    let template_path = args.template.as_deref().ok_or_else(|| {
        CliRequestError::Usage("transform requires --template or --config".into())
    })?;
    let data = engine_input(
        context,
        data_path,
        None,
        transform_data_scope(args, data_path),
    )
    .map_err(CliRequestError::Engine)?;
    let template = template_input(
        context,
        template_path,
        transform_template_scope(args, template_path),
    )
    .map_err(CliRequestError::Engine)?;
    let template_identity = template
        .identity
        .clone()
        .unwrap_or_else(|| template.root_scope.format_identity());
    let template_kind = eng::classify_transform_template_identity_with_registry(
        &template_identity,
        &context.template_adapter_registry,
    )
    .map_err(|error| CliRequestError::Usage(error.to_string()))?;
    let template_entrypoint = transform_template_entrypoint(args.template_entrypoint.as_deref());
    let params = transform_parse_cli_params(&args.params)?;
    validate_transform_template_module_surface(template_kind, &template_entrypoint, &params)?;
    let execution_policy =
        transform_execution_policy_for(template_kind, &template_entrypoint, &params);
    let target_scope = transform_target_scope(args);
    Ok(eng::TransformRequest {
        data,
        template,
        template_kind,
        template_entrypoint,
        params,
        preserve_source_offsets: false,
        context: context.clone(),
        target: target_scope.format_identity_option(),
        target_scope,
        scheduler_scope_ids: eng::TransformSchedulerScopeIds {
            data_load: 0,
            template_load: 1,
            execution: 2,
            output: 3,
        },
        execution_policy,
    })
}

fn transform_graph_config_from_args(
    context: &eng::EngineContext,
    args: &cli::TransformArgs,
) -> Result<(TransformGraphConfig, String, Option<PathBuf>), CliRequestError> {
    let config_path = args.config.as_ref().ok_or_else(|| {
        CliRequestError::Usage("transform graph execution requires --config".into())
    })?;
    let local_config_path = local_path_or_file_uri(config_path, "config path")
        .ok()
        .map(|path| path.into_owned());
    let config_source_uri = config_path.display().to_string();
    let read = read_source(
        context,
        config_path,
        "config path",
        ResolvePurpose::Config,
        args.config_content_type.as_deref(),
    )
    .map_err(|source| {
        CliRequestError::Engine(EngineError::Io {
            path: config_path.clone(),
            source,
        })
    })?;
    let identity = eng::FormatIdentity {
        content_type: args
            .config_content_type
            .clone()
            .or_else(|| {
                local_config_path
                    .as_ref()
                    .and_then(|path| infer_config_content_type(path))
            })
            .or_else(|| infer_config_content_type(config_path))
            .or_else(|| read.content_type.clone()),
        schema: Some(
            args.config_schema
                .clone()
                .unwrap_or_else(|| transform_config::TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
        ),
        default_namespace: None,
        namespaces: BTreeMap::new(),
        base_uri: Some(config_source_uri.clone()),
    };
    let response = transform_config::parse_transform_graph_config(
        transform_config::TransformGraphParseRequest {
            bytes: read.bytes,
            identity,
            base_uri: Some(config_source_uri.clone()),
        },
    )
    .map_err(|error| CliRequestError::RunConfigDiagnostics {
        config: None,
        diagnostics: vec![run_config_error_diagnostic(
            error.code,
            error.message,
            Some(config_source_uri.clone()),
        )],
    })?;
    if !response.diagnostics.is_empty() {
        return Err(CliRequestError::RunConfigDiagnostics {
            config: None,
            diagnostics: response.diagnostics,
        });
    }
    Ok((response.graph, config_source_uri, local_config_path))
}

fn transform_graph_path(raw: &str, config_local_path: Option<&Path>) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() || uri_scheme(raw).is_some() && !is_windows_drive_path(raw) {
        return path.to_path_buf();
    }
    config_local_path
        .and_then(Path::parent)
        .map(|parent| parent.join(path))
        .unwrap_or_else(|| path.to_path_buf())
}

#[derive(Debug, Clone)]
struct TransformGraphArtifactVariant {
    id: String,
    bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct TransformGraphImportMatch {
    path: PathBuf,
    bindings: BTreeMap<String, String>,
}

type TransformGraphJoinGroup = (
    String,
    Vec<(String, TransformGraphArtifactVariant)>,
    BTreeMap<String, String>,
);

fn transform_config_diagnostic(
    uri: &str,
    code: &str,
    message: impl Into<String>,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(uri.to_owned()),
        code: code.to_owned(),
        severity: cem_ml::diagnostics::Severity::Fatal,
        message: message.into(),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn transform_config_error(uri: &str, code: &str, message: impl Into<String>) -> CliRequestError {
    CliRequestError::RunConfigDiagnostics {
        config: None,
        diagnostics: vec![transform_config_diagnostic(uri, code, message)],
    }
}

fn path_display_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn transform_graph_binding_path(path: &Path, config_local_path: Option<&Path>) -> PathBuf {
    if let Some(config_dir) = config_local_path.and_then(Path::parent) {
        if let Ok(relative) = path.strip_prefix(config_dir) {
            return relative.to_path_buf();
        }
    }
    path.to_path_buf()
}

fn transform_graph_glob_parent(pattern_path: &Path) -> PathBuf {
    pattern_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn transform_graph_split_recursive_glob_parent(parent: &Path) -> Option<(PathBuf, PathBuf)> {
    let recursive = OsStr::new("**");
    let mut root = PathBuf::new();
    let mut suffix = PathBuf::new();
    let mut seen_recursive = false;

    for component in parent.components() {
        if component.as_os_str() == recursive {
            seen_recursive = true;
            continue;
        }
        if seen_recursive {
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

fn transform_graph_file_name_matches(file_name: &str, prefix: &str, suffix: &str) -> bool {
    file_name.starts_with(prefix) && file_name.ends_with(suffix)
}

fn transform_graph_collect_one_level_import_glob_matches(
    parent: &Path,
    prefix: &str,
    suffix: &str,
) -> io::Result<Vec<PathBuf>> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if transform_graph_file_name_matches(&file_name, prefix, suffix) {
            matches.push(entry.path());
        }
    }
    Ok(matches)
}

fn transform_graph_collect_recursive_import_glob_matches(
    current: &Path,
    suffix_dir: &Path,
    prefix: &str,
    suffix: &str,
    matches: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let candidate_parent = if suffix_dir.as_os_str().is_empty() {
        current.to_path_buf()
    } else {
        current.join(suffix_dir)
    };
    if candidate_parent.is_dir() {
        matches.extend(transform_graph_collect_one_level_import_glob_matches(
            &candidate_parent,
            prefix,
            suffix,
        )?);
    }

    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            transform_graph_collect_recursive_import_glob_matches(
                &entry.path(),
                suffix_dir,
                prefix,
                suffix,
                matches,
            )?;
        }
    }
    Ok(())
}

fn transform_graph_collect_import_glob_matches(
    parent: &Path,
    prefix: &str,
    suffix: &str,
) -> io::Result<Vec<PathBuf>> {
    let mut matches =
        if let Some((root, suffix_dir)) = transform_graph_split_recursive_glob_parent(parent) {
            let mut matches = Vec::new();
            transform_graph_collect_recursive_import_glob_matches(
                &root,
                &suffix_dir,
                prefix,
                suffix,
                &mut matches,
            )?;
            matches
        } else {
            transform_graph_collect_one_level_import_glob_matches(parent, prefix, suffix)?
        };
    matches.sort();
    Ok(matches)
}

fn transform_graph_source_bindings(
    path: &Path,
    config_local_path: Option<&Path>,
    index: usize,
) -> BTreeMap<String, String> {
    let binding_path = transform_graph_binding_path(path, config_local_path);
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
    let dir = binding_path
        .parent()
        .map(path_display_slash)
        .unwrap_or_default();

    BTreeMap::from([
        ("src".to_owned(), path.display().to_string()),
        ("path".to_owned(), path_display_slash(&binding_path)),
        ("dir".to_owned(), dir),
        ("file".to_owned(), file),
        ("stem".to_owned(), stem),
        ("ext".to_owned(), ext),
        ("index".to_owned(), index.to_string()),
    ])
}

fn transform_graph_expand_import_paths(
    context: &eng::EngineContext,
    raw: &str,
    config_local_path: Option<&Path>,
    config_source_uri: &str,
) -> Result<Vec<TransformGraphImportMatch>, CliRequestError> {
    if !raw.contains('*') {
        let path = transform_graph_path(raw, config_local_path);
        return Ok(vec![TransformGraphImportMatch {
            bindings: transform_graph_source_bindings(&path, config_local_path, 0),
            path,
        }]);
    }

    if uri_scheme(raw).is_some() && !is_windows_drive_path(raw) {
        return transform_graph_expand_resolver_import_paths(context, raw, config_source_uri);
    }

    transform_graph_validate_import_glob(raw, config_source_uri)?;
    let pattern_path = transform_graph_path(raw, config_local_path);
    let pattern_file = pattern_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();

    let parent = transform_graph_glob_parent(&pattern_path);

    let (prefix, suffix) = pattern_file.split_once('*').unwrap_or(("", ""));
    let matches =
        transform_graph_collect_import_glob_matches(&parent, prefix, suffix).map_err(|source| {
            CliRequestError::Engine(EngineError::Io {
                path: parent.clone(),
                source,
            })
        })?;
    if matches.is_empty() {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.import_glob_empty",
            format!("import glob `{raw}` matched no files"),
        ));
    }

    Ok(matches
        .into_iter()
        .enumerate()
        .map(|(index, path)| TransformGraphImportMatch {
            bindings: transform_graph_source_bindings(&path, config_local_path, index),
            path,
        })
        .collect())
}

fn transform_graph_expand_resolver_import_paths(
    context: &eng::EngineContext,
    raw: &str,
    config_source_uri: &str,
) -> Result<Vec<TransformGraphImportMatch>, CliRequestError> {
    transform_graph_validate_import_glob(raw, config_source_uri)?;
    let request = ResolveListRequest::new(raw, ResolvePurpose::Input)
        .with_max_entries(TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES + 1);
    let mut entries = match context.resolver_registry.list(&request) {
        Ok(entries) => entries,
        Err(ResolverDiagnostic::UnsupportedResolver { .. }) => {
            return Err(transform_config_error(
                config_source_uri,
                "cem.transform_config.import_glob_resolver_unsupported",
                format!("import glob `{raw}` requires a resolver with list support"),
            ));
        }
        Err(error) => {
            return Err(transform_config_error(
                config_source_uri,
                "cem.transform_config.import_glob_resolver_error",
                format!("import glob `{raw}` failed during resolver listing: {error}"),
            ));
        }
    };
    entries.sort_by(|left, right| left.uri.cmp(&right.uri));
    if entries.is_empty() {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.import_glob_empty",
            format!("import glob `{raw}` matched no files"),
        ));
    }
    if entries.len() > TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.import_glob_too_many",
            format!(
                "import glob `{raw}` matched more than {TRANSFORM_GRAPH_IMPORT_GLOB_MAX_ENTRIES} files"
            ),
        ));
    }

    Ok(entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let path = PathBuf::from(&entry.uri);
            TransformGraphImportMatch {
                bindings: transform_graph_source_bindings(&path, None, index),
                path,
            }
        })
        .collect())
}

fn transform_graph_validate_import_glob(
    raw: &str,
    config_source_uri: &str,
) -> Result<(), CliRequestError> {
    let (dir, file) = raw.rsplit_once('/').unwrap_or(("", raw));
    if file.matches('*').count() != 1 {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.import_glob_unsupported",
            format!("import glob `{raw}` must contain exactly one `*` in the file name"),
        ));
    }
    let mut recursive_segments = 0;
    for segment in dir.split('/') {
        if segment == "**" {
            recursive_segments += 1;
            continue;
        }
        if segment.contains('*') {
            return Err(transform_config_error(
                config_source_uri,
                "cem.transform_config.import_glob_unsupported",
                format!("import glob `{raw}` can only use `**` as a complete directory segment"),
            ));
        }
    }
    if recursive_segments > 1 {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.import_glob_unsupported",
            format!("import glob `{raw}` can contain at most one `**` directory segment"),
        ));
    }
    Ok(())
}

fn transform_graph_variant_id(base: &str, index: usize, count: usize) -> String {
    if count == 1 {
        base.to_owned()
    } else {
        format!("{base}:{index}")
    }
}

fn transform_graph_expand_path_template(
    template: &str,
    bindings: &BTreeMap<String, String>,
    config_source_uri: &str,
) -> Result<String, CliRequestError> {
    transform_graph_expand_binding_template(
        template,
        bindings,
        config_source_uri,
        "output path template",
        "output_binding",
    )
}

fn transform_graph_expand_param_template(
    template: &str,
    bindings: &BTreeMap<String, String>,
    config_source_uri: &str,
) -> Result<String, CliRequestError> {
    transform_graph_expand_binding_template(
        template,
        bindings,
        config_source_uri,
        "param value template",
        "param_binding",
    )
}

fn transform_graph_expand_binding_template(
    template: &str,
    bindings: &BTreeMap<String, String>,
    config_source_uri: &str,
    label: &str,
    code_prefix: &str,
) -> Result<String, CliRequestError> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            return Err(transform_config_error(
                config_source_uri,
                &format!("cem.transform_config.{code_prefix}_unclosed"),
                format!("{label} `{template}` has an unclosed binding"),
            ));
        };
        let name = &after_open[..close];
        if name.trim().is_empty() {
            return Err(transform_config_error(
                config_source_uri,
                &format!("cem.transform_config.{code_prefix}_empty"),
                format!("{label} `{template}` has an empty binding"),
            ));
        }
        let Some(value) = bindings.get(name) else {
            return Err(transform_config_error(
                config_source_uri,
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

fn transform_graph_stage_params(
    node: &TransformGraphNode,
    bindings: &BTreeMap<String, String>,
    config_source_uri: &str,
) -> Result<BTreeMap<String, serde_json::Value>, CliRequestError> {
    let mut params = BTreeMap::new();
    for (name, value) in &node.params {
        let value = transform_graph_expand_param_template(value, bindings, config_source_uri)?;
        params.insert(name.clone(), serde_json::Value::String(value));
    }
    Ok(params)
}

fn transform_graph_importmap_imports(
    context: &eng::EngineContext,
    raw_path: &str,
    bindings: &BTreeMap<String, String>,
    config_local_path: Option<&Path>,
    config_source_uri: &str,
    required: bool,
) -> Result<BTreeMap<String, String>, CliRequestError> {
    let expanded = transform_graph_expand_param_template(raw_path, bindings, config_source_uri)?;
    let path = transform_graph_path(&expanded, config_local_path);
    let read = read_source(
        context,
        &path,
        "importmap URI",
        ResolvePurpose::ModuleMap,
        Some("application/importmap+json"),
    )
    .map_err(|source| {
        CliRequestError::Engine(EngineError::Io {
            path: path.clone(),
            source,
        })
    })?;
    let value: serde_json::Value = serde_json::from_slice(&read.bytes).map_err(|error| {
        CliRequestError::Usage(format!(
            "importmap `{}` is not valid JSON: {error}",
            path.display()
        ))
    })?;
    let Some(imports) = value.get("imports").and_then(serde_json::Value::as_object) else {
        if required {
            return Err(CliRequestError::Usage(format!(
                "importmap `{}` requires an object `imports` map",
                path.display()
            )));
        }
        return Ok(BTreeMap::new());
    };
    let mut entries = BTreeMap::new();
    for (key, value) in imports {
        let Some(target) = value.as_str() else {
            return Err(CliRequestError::Usage(format!(
                "importmap `{}` entry `{key}` must be a string",
                path.display()
            )));
        };
        entries.insert(key.clone(), target.to_owned());
    }
    Ok(entries)
}

fn transform_graph_scope(
    content_type: Option<String>,
    schema: Option<String>,
    path: &Path,
) -> ScopeConfig {
    ScopeConfig {
        default_content_type: content_type
            .or_else(|| run_config::infer_content_type_from_path(&path.display().to_string())),
        schema,
        ..ScopeConfig::default()
    }
}

fn transform_graph_primary_ref(
    graph: &TransformGraphConfig,
    node: &TransformGraphNode,
) -> Result<(String, TransformGraphEdgeRole), CliRequestError> {
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
            CliRequestError::Usage(format!(
                "transform graph node `{}` requires an input edge",
                node.id
            ))
        })
}

fn transform_graph_variants_for_ref(
    variants: &BTreeMap<String, Vec<TransformGraphArtifactVariant>>,
    owner_id: &str,
    field: &str,
    target: &str,
) -> Result<Vec<TransformGraphArtifactVariant>, CliRequestError> {
    variants.get(target).cloned().ok_or_else(|| {
        CliRequestError::Usage(format!(
            "transform graph node `{owner_id}` references unknown artifact `{target}` via `{field}`"
        ))
    })
}

fn transform_graph_single_variant_for_ref(
    variants: &BTreeMap<String, Vec<TransformGraphArtifactVariant>>,
    owner_id: &str,
    field: &str,
    target: &str,
    config_source_uri: &str,
) -> Result<TransformGraphArtifactVariant, CliRequestError> {
    let matches = transform_graph_variants_for_ref(variants, owner_id, field, target)?;
    if matches.len() != 1 {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.join_multi_artifact_unsupported",
            format!(
                "node `{owner_id}` references multi-artifact `{target}` via `{field}`; explicit join semantics are not implemented"
            ),
        ));
    }
    Ok(matches[0].clone())
}

fn to_engine_dependency_role(role: TransformGraphEdgeRole) -> eng::TransformGraphDependencyRole {
    match role {
        TransformGraphEdgeRole::Parent => eng::TransformGraphDependencyRole::Parent,
        TransformGraphEdgeRole::Input => eng::TransformGraphDependencyRole::PrimaryInput,
        TransformGraphEdgeRole::With => eng::TransformGraphDependencyRole::SecondaryInput,
    }
}

fn to_engine_join_mode(mode: TransformGraphJoinMode) -> eng::TransformGraphJoinMode {
    match mode {
        TransformGraphJoinMode::Collect => eng::TransformGraphJoinMode::Collect,
        TransformGraphJoinMode::GroupBy => eng::TransformGraphJoinMode::GroupBy,
        TransformGraphJoinMode::MatchBy => eng::TransformGraphJoinMode::MatchBy,
        TransformGraphJoinMode::Zip => eng::TransformGraphJoinMode::Zip,
    }
}

fn to_engine_importmap_rewrite_mode(
    mode: transform_config::TransformGraphImportMapRewriteMode,
) -> eng::TransformGraphImportMapRewriteMode {
    match mode {
        transform_config::TransformGraphImportMapRewriteMode::ReplaceImports => {
            eng::TransformGraphImportMapRewriteMode::ReplaceImports
        }
        transform_config::TransformGraphImportMapRewriteMode::Merge => {
            eng::TransformGraphImportMapRewriteMode::Merge
        }
        transform_config::TransformGraphImportMapRewriteMode::ReplaceScript => {
            eng::TransformGraphImportMapRewriteMode::ReplaceScript
        }
    }
}

fn to_engine_importmap_missing_policy(
    policy: transform_config::TransformGraphImportMapMissingPolicy,
) -> eng::TransformGraphImportMapMissingPolicy {
    match policy {
        transform_config::TransformGraphImportMapMissingPolicy::Error => {
            eng::TransformGraphImportMapMissingPolicy::Error
        }
        transform_config::TransformGraphImportMapMissingPolicy::Ignore => {
            eng::TransformGraphImportMapMissingPolicy::Ignore
        }
        transform_config::TransformGraphImportMapMissingPolicy::Insert => {
            eng::TransformGraphImportMapMissingPolicy::Insert
        }
    }
}

fn transform_graph_join_by(
    node: &TransformGraphNode,
    config_source_uri: &str,
) -> Result<String, CliRequestError> {
    let by = node.join_by.as_deref().unwrap_or("").trim();
    if by.is_empty() {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.join_by_missing",
            format!("join node `{}` with keyed `@mode` requires `@by`", node.id),
        ));
    }
    if matches!(by, "count" | "key") {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.join_by_reserved",
            format!(
                "join node `{}` uses reserved grouping binding `{by}`",
                node.id
            ),
        ));
    }
    Ok(by.to_owned())
}

fn transform_graph_group_by_join_groups(
    node: &TransformGraphNode,
    input_variants: Vec<TransformGraphArtifactVariant>,
    config_source_uri: &str,
) -> Result<Vec<TransformGraphJoinGroup>, CliRequestError> {
    let by = transform_graph_join_by(node, config_source_uri)?;

    let mut groups: BTreeMap<String, Vec<TransformGraphArtifactVariant>> = BTreeMap::new();
    for variant in input_variants {
        let Some(value) = variant.bindings.get(&by) else {
            return Err(transform_config_error(
                config_source_uri,
                "cem.transform_config.join_by_unknown",
                format!(
                    "join node `{}` groups by unknown binding `{by}` on artifact `{}`",
                    node.id, variant.id
                ),
            ));
        };
        groups.entry(value.clone()).or_default().push(variant);
    }

    let group_count = groups.len();
    Ok(groups
        .into_iter()
        .enumerate()
        .map(|(index, (key, variants))| {
            let id = transform_graph_variant_id(&node.id, index, group_count);
            let mut bindings = BTreeMap::from([
                ("count".to_owned(), variants.len().to_string()),
                ("key".to_owned(), key.clone()),
            ]);
            bindings.insert(by.to_owned(), key);
            let inputs = variants
                .into_iter()
                .map(|variant| ("primary".to_owned(), variant))
                .collect();
            (id, inputs, bindings)
        })
        .collect())
}

fn transform_graph_match_by_join_groups(
    node: &TransformGraphNode,
    primary_variants: Vec<TransformGraphArtifactVariant>,
    variants: &BTreeMap<String, Vec<TransformGraphArtifactVariant>>,
    config_source_uri: &str,
) -> Result<Vec<TransformGraphJoinGroup>, CliRequestError> {
    let by = transform_graph_join_by(node, config_source_uri)?;
    if node.with.is_empty() {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.join_with_missing",
            format!(
                "join node `{}` with `@mode=\"match-by\"` requires at least one `@with:*` input",
                node.id
            ),
        ));
    }

    let mut primary_groups: BTreeMap<String, Vec<TransformGraphArtifactVariant>> = BTreeMap::new();
    for variant in primary_variants {
        let Some(value) = variant.bindings.get(&by) else {
            return Err(transform_config_error(
                config_source_uri,
                "cem.transform_config.join_by_unknown",
                format!(
                    "join node `{}` matches by unknown binding `{by}` on artifact `{}`",
                    node.id, variant.id
                ),
            ));
        };
        primary_groups
            .entry(value.clone())
            .or_default()
            .push(variant);
    }

    let mut secondary_groups = BTreeMap::new();
    for (name, target) in &node.with {
        let secondary_variants =
            transform_graph_variants_for_ref(variants, &node.id, &format!("with:{name}"), target)?;
        let mut by_key: BTreeMap<String, Vec<TransformGraphArtifactVariant>> = BTreeMap::new();
        for variant in secondary_variants {
            let Some(value) = variant.bindings.get(&by) else {
                return Err(transform_config_error(
                    config_source_uri,
                    "cem.transform_config.join_by_unknown",
                    format!(
                        "join node `{}` matches by unknown binding `{by}` on artifact `{}`",
                        node.id, variant.id
                    ),
                ));
            };
            by_key.entry(value.clone()).or_default().push(variant);
        }
        secondary_groups.insert(name.clone(), by_key);
    }

    let group_count = primary_groups.len();
    Ok(primary_groups
        .into_iter()
        .enumerate()
        .map(|(index, (key, primary))| {
            let id = transform_graph_variant_id(&node.id, index, group_count);
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

fn transform_graph_zip_join_groups(
    node: &TransformGraphNode,
    primary_variants: Vec<TransformGraphArtifactVariant>,
    variants: &BTreeMap<String, Vec<TransformGraphArtifactVariant>>,
    config_source_uri: &str,
) -> Result<Vec<TransformGraphJoinGroup>, CliRequestError> {
    if node.with.is_empty() {
        return Err(transform_config_error(
            config_source_uri,
            "cem.transform_config.join_with_missing",
            format!(
                "join node `{}` with `@mode=\"zip\"` requires at least one `@with:*` input",
                node.id
            ),
        ));
    }

    let primary_count = primary_variants.len();
    let mut secondary_inputs = BTreeMap::new();
    for (name, target) in &node.with {
        let secondary_variants =
            transform_graph_variants_for_ref(variants, &node.id, &format!("with:{name}"), target)?;
        if secondary_variants.len() != primary_count {
            return Err(transform_config_error(
                config_source_uri,
                "cem.transform_config.join_zip_count_mismatch",
                format!(
                    "join node `{}` cannot zip primary input count {} with `@with:{name}` count {}; zip joins require equal input counts",
                    node.id,
                    primary_count,
                    secondary_variants.len()
                ),
            ));
        }
        secondary_inputs.insert(name.clone(), secondary_variants);
    }

    Ok(primary_variants
        .into_iter()
        .enumerate()
        .map(|(index, primary)| {
            let id = transform_graph_variant_id(&node.id, index, primary_count);
            let mut inputs = vec![("primary".to_owned(), primary)];
            for (name, secondary_variants) in &secondary_inputs {
                inputs.push((name.clone(), secondary_variants[index].clone()));
            }
            let bindings = BTreeMap::from([
                ("count".to_owned(), inputs.len().to_string()),
                ("index".to_owned(), index.to_string()),
            ]);
            (id, inputs, bindings)
        })
        .collect())
}

fn transform_graph_request_from_config(
    context: &eng::EngineContext,
    graph: &TransformGraphConfig,
    config_local_path: Option<&Path>,
    config_source_uri: &str,
) -> Result<eng::TransformGraphRequest, CliRequestError> {
    let mut imports = Vec::new();
    let mut joins = Vec::new();
    let mut stages = Vec::new();
    let mut importmap_rewrites = Vec::new();
    let mut exports = Vec::new();
    let mut edges = Vec::new();
    let mut variants: BTreeMap<String, Vec<TransformGraphArtifactVariant>> = BTreeMap::new();
    let mut next_scope_id = 0u32;
    for node in &graph.nodes {
        match node.kind {
            TransformGraphNodeKind::Import => {
                let src = node.src.as_ref().ok_or_else(|| {
                    CliRequestError::Usage(format!("import node `{}` requires @src", node.id))
                })?;
                let matches = transform_graph_expand_import_paths(
                    context,
                    src,
                    config_local_path,
                    config_source_uri,
                )?;
                let match_count = matches.len();
                let mut import_variants = Vec::new();
                for (index, import_match) in matches.into_iter().enumerate() {
                    let import_id = transform_graph_variant_id(&node.id, index, match_count);
                    let scope = transform_graph_scope(
                        node.content_type.clone(),
                        node.schema.clone(),
                        &import_match.path,
                    );
                    let input = engine_input(context, &import_match.path, None, scope)
                        .map_err(CliRequestError::Engine)?;
                    imports.push(eng::TransformGraphImport {
                        id: import_id.clone(),
                        input,
                        scheduler_scope_id: next_scope_id,
                    });
                    next_scope_id += 1;
                    import_variants.push(TransformGraphArtifactVariant {
                        id: import_id,
                        bindings: import_match.bindings,
                    });
                }
                variants.insert(node.id.clone(), import_variants);
            }
            TransformGraphNodeKind::Join => {
                let mode = node.join_mode.ok_or_else(|| {
                    CliRequestError::Usage(format!("join node `{}` requires @mode", node.id))
                })?;
                let (input_ref, input_role) = transform_graph_primary_ref(graph, node)?;
                let input_variants =
                    transform_graph_variants_for_ref(&variants, &node.id, "input", &input_ref)?;
                let grouped_variants = match mode {
                    TransformGraphJoinMode::Collect => {
                        let input_count = input_variants.len();
                        let inputs = input_variants
                            .into_iter()
                            .map(|variant| ("primary".to_owned(), variant))
                            .collect();
                        vec![(
                            node.id.clone(),
                            inputs,
                            BTreeMap::from([("count".to_owned(), input_count.to_string())]),
                        )]
                    }
                    TransformGraphJoinMode::GroupBy => transform_graph_group_by_join_groups(
                        node,
                        input_variants,
                        config_source_uri,
                    )?,
                    TransformGraphJoinMode::MatchBy => transform_graph_match_by_join_groups(
                        node,
                        input_variants,
                        &variants,
                        config_source_uri,
                    )?,
                    TransformGraphJoinMode::Zip => transform_graph_zip_join_groups(
                        node,
                        input_variants,
                        &variants,
                        config_source_uri,
                    )?,
                };
                let mut output_variants = Vec::new();
                for (join_id, input_variants, bindings) in grouped_variants {
                    let input_names = if matches!(
                        mode,
                        TransformGraphJoinMode::MatchBy | TransformGraphJoinMode::Zip
                    ) {
                        std::iter::once("primary".to_owned())
                            .chain(node.with.keys().cloned())
                            .collect::<Vec<_>>()
                    } else {
                        input_variants
                            .iter()
                            .map(|(input_name, _)| input_name.clone())
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect::<Vec<_>>()
                    };
                    let join_inputs = input_variants
                        .iter()
                        .map(|(input_name, variant)| eng::TransformGraphJoinInput {
                            input_name: input_name.clone(),
                            artifact_id: variant.id.clone(),
                            bindings: variant.bindings.clone(),
                        })
                        .collect::<Vec<_>>();
                    joins.push(eng::TransformGraphJoin {
                        id: join_id.clone(),
                        mode: to_engine_join_mode(mode),
                        input_names,
                        inputs: join_inputs,
                        bindings: bindings.clone(),
                        scheduler_scope_id: next_scope_id,
                    });
                    next_scope_id += 1;
                    for (input_name, variant) in input_variants {
                        let role = if input_name == "primary" {
                            to_engine_dependency_role(input_role)
                        } else {
                            eng::TransformGraphDependencyRole::SecondaryInput
                        };
                        edges.push(eng::TransformGraphDependency {
                            from: variant.id,
                            to: join_id.clone(),
                            role,
                        });
                    }
                    output_variants.push(TransformGraphArtifactVariant {
                        id: join_id,
                        bindings,
                    });
                }
                variants.insert(node.id.clone(), output_variants);
            }
            TransformGraphNodeKind::Transform => {
                let src = node.src.as_ref().ok_or_else(|| {
                    CliRequestError::Usage(format!(
                        "transform node `{}` requires template @src",
                        node.id
                    ))
                })?;
                let path = transform_graph_path(src, config_local_path);
                let scope = transform_graph_scope(
                    node.template_content_type.clone(),
                    node.template_schema.clone(),
                    &path,
                );
                let template =
                    template_input(context, &path, scope).map_err(CliRequestError::Engine)?;
                let template_identity = template
                    .identity
                    .clone()
                    .unwrap_or_else(|| template.root_scope.format_identity());
                let template_kind = match node.template_kind {
                    Some(kind) => kind,
                    None => eng::classify_transform_template_identity_with_registry(
                        &template_identity,
                        &context.template_adapter_registry,
                    )
                    .map_err(|error| CliRequestError::Usage(error.to_string()))?,
                };
                let (primary_ref, primary_role) = transform_graph_primary_ref(graph, node)?;
                let primary_variants =
                    transform_graph_variants_for_ref(&variants, &node.id, "input", &primary_ref)?;
                let variant_count = primary_variants.len();
                let mut stage_variants = Vec::new();
                for (index, primary_variant) in primary_variants.into_iter().enumerate() {
                    let stage_id = transform_graph_variant_id(&node.id, index, variant_count);
                    let template_load = next_scope_id;
                    let execution = next_scope_id + 1;
                    next_scope_id += 2;
                    let template_entrypoint =
                        transform_template_entrypoint(node.entrypoint.as_deref());
                    let params = transform_graph_stage_params(
                        node,
                        &primary_variant.bindings,
                        config_source_uri,
                    )?;
                    validate_transform_template_module_surface(
                        template_kind,
                        &template_entrypoint,
                        &params,
                    )?;
                    let execution_policy = transform_execution_policy_for(
                        template_kind,
                        &template_entrypoint,
                        &params,
                    );
                    let mut secondary_inputs = BTreeMap::new();
                    for (name, target) in &node.with {
                        let secondary = transform_graph_single_variant_for_ref(
                            &variants,
                            &node.id,
                            &format!("with:{name}"),
                            target,
                            config_source_uri,
                        )?;
                        secondary_inputs.insert(name.clone(), secondary.id.clone());
                        edges.push(eng::TransformGraphDependency {
                            from: secondary.id,
                            to: stage_id.clone(),
                            role: eng::TransformGraphDependencyRole::SecondaryInput,
                        });
                    }
                    stages.push(eng::TransformGraphStage {
                        id: stage_id.clone(),
                        template: template.clone(),
                        template_kind,
                        template_entrypoint,
                        params,
                        execution_policy,
                        primary_input: primary_variant.id.clone(),
                        secondary_inputs,
                        scheduler_scope_ids: eng::TransformStageSchedulerScopeIds {
                            template_load,
                            execution,
                        },
                    });
                    edges.push(eng::TransformGraphDependency {
                        from: primary_variant.id,
                        to: stage_id.clone(),
                        role: to_engine_dependency_role(primary_role),
                    });
                    stage_variants.push(TransformGraphArtifactVariant {
                        id: stage_id,
                        bindings: primary_variant.bindings,
                    });
                }
                variants.insert(node.id.clone(), stage_variants);
            }
            TransformGraphNodeKind::ImportMapRewrite => {
                let target_map = node.target_map.as_ref().ok_or_else(|| {
                    CliRequestError::Usage(format!(
                        "rewrite-importmap node `{}` requires @target-map",
                        node.id
                    ))
                })?;
                let (primary_ref, primary_role) = transform_graph_primary_ref(graph, node)?;
                let primary_variants =
                    transform_graph_variants_for_ref(&variants, &node.id, "input", &primary_ref)?;
                let variant_count = primary_variants.len();
                let mut rewrite_variants = Vec::new();
                for (index, primary_variant) in primary_variants.into_iter().enumerate() {
                    let rewrite_id = transform_graph_variant_id(&node.id, index, variant_count);
                    let source_imports = if let Some(source_map) = node.source_map.as_deref() {
                        transform_graph_importmap_imports(
                            context,
                            source_map,
                            &primary_variant.bindings,
                            config_local_path,
                            config_source_uri,
                            false,
                        )?
                    } else {
                        BTreeMap::new()
                    };
                    let target_imports = transform_graph_importmap_imports(
                        context,
                        target_map,
                        &primary_variant.bindings,
                        config_local_path,
                        config_source_uri,
                        true,
                    )?;
                    importmap_rewrites.push(eng::TransformGraphImportMapRewrite {
                        id: rewrite_id.clone(),
                        primary_input: primary_variant.id.clone(),
                        source_imports,
                        target_imports,
                        mode: to_engine_importmap_rewrite_mode(node.rewrite_mode.unwrap_or(
                            transform_config::TransformGraphImportMapRewriteMode::ReplaceImports,
                        )),
                        missing_policy: to_engine_importmap_missing_policy(
                            node.missing_policy.unwrap_or(
                                transform_config::TransformGraphImportMapMissingPolicy::Error,
                            ),
                        ),
                        scheduler_scope_id: next_scope_id,
                    });
                    next_scope_id += 1;
                    edges.push(eng::TransformGraphDependency {
                        from: primary_variant.id,
                        to: rewrite_id.clone(),
                        role: to_engine_dependency_role(primary_role),
                    });
                    rewrite_variants.push(TransformGraphArtifactVariant {
                        id: rewrite_id,
                        bindings: primary_variant.bindings,
                    });
                }
                variants.insert(node.id.clone(), rewrite_variants);
            }
            TransformGraphNodeKind::Export => {
                let out = node.out.as_ref().ok_or_else(|| {
                    CliRequestError::Usage(format!("export node `{}` requires @out", node.id))
                })?;
                let (input_ref, input_role) = transform_graph_primary_ref(graph, node)?;
                let input_variants =
                    transform_graph_variants_for_ref(&variants, &node.id, "input", &input_ref)?;
                let variant_count = input_variants.len();
                for (index, input_variant) in input_variants.into_iter().enumerate() {
                    let export_id = transform_graph_variant_id(&node.id, index, variant_count);
                    let resolved_out = transform_graph_expand_path_template(
                        out,
                        &input_variant.bindings,
                        config_source_uri,
                    )?;
                    let path = transform_graph_path(&resolved_out, config_local_path);
                    let target_scope = transform_graph_scope(
                        node.content_type.clone(),
                        node.schema.clone(),
                        &path,
                    );
                    let target = target_scope.format_identity_option();
                    exports.push(eng::TransformGraphExport {
                        id: export_id.clone(),
                        input: input_variant.id.clone(),
                        destination: Some(path.display().to_string()),
                        target,
                        target_scope,
                        scheduler_scope_id: next_scope_id,
                    });
                    next_scope_id += 1;
                    edges.push(eng::TransformGraphDependency {
                        from: input_variant.id,
                        to: export_id,
                        role: to_engine_dependency_role(input_role),
                    });
                }
            }
        }
    }

    Ok(eng::TransformGraphRequest {
        imports,
        joins,
        stages,
        importmap_rewrites,
        exports,
        edges,
        preserve_source_offsets: false,
        context: context.clone(),
        execution_policy: eng::TransformExecutionPolicy::default(),
    })
}

fn transform_graph_request_from_args(
    context: &eng::EngineContext,
    args: &cli::TransformArgs,
) -> Result<(eng::TransformGraphRequest, String), CliRequestError> {
    let (graph, config_source_uri, local_config_path) =
        transform_graph_config_from_args(context, args)?;
    let request = transform_graph_request_from_config(
        context,
        &graph,
        local_config_path.as_deref(),
        &config_source_uri,
    )?;
    Ok((request, config_source_uri))
}

fn convert_target_scope_with_config(args: &cli::ConvertArgs, config: &RunConfig) -> ScopeConfig {
    config
        .outputs
        .first()
        .map(|output| output.root_scope.clone())
        .unwrap_or_else(|| convert_target_scope(args))
}

fn convert_output_destination(args: &cli::ConvertArgs, config: &RunConfig) -> Option<PathBuf> {
    args.out.clone().or_else(|| {
        config
            .outputs
            .first()
            .and_then(|output: &OutputSpec| output.destination.as_ref())
            .map(PathBuf::from)
    })
}

fn convert_configured_inputs(
    context: &eng::EngineContext,
    args: &cli::ConvertArgs,
    config: &RunConfig,
    positional_defaults: &ScopeConfig,
) -> Result<Vec<eng::EngineInput>, CliRequestError> {
    let mut inputs = Vec::new();
    if let Some(path) = args.input.as_deref() {
        inputs.push(
            engine_input(
                context,
                path,
                args.from_format,
                positional_input_scope(path, positional_defaults),
            )
            .map_err(CliRequestError::Engine)?,
        );
    }
    for spec in &config.inputs {
        inputs.push(engine_input_from_spec(context, spec, args.from_format)?);
    }
    Ok(inputs)
}

fn convert_input_for_output(
    output: &OutputSpec,
    output_index: usize,
    inputs: &[eng::EngineInput],
) -> Result<eng::EngineInput, CliRequestError> {
    if let Some(input_ref) = output.input_ref.as_deref() {
        return inputs
            .iter()
            .find(|input| input.uri == input_ref)
            .cloned()
            .ok_or_else(|| {
                CliRequestError::Usage(format!(
                    "output spec at index {output_index} references unknown input `{input_ref}`"
                ))
            });
    }

    match inputs {
        [input] => Ok(input.clone()),
        [] => Err(CliRequestError::Usage(
            "convert requires one input, --input-spec, or output input references".to_owned(),
        )),
        _ => Err(CliRequestError::Usage(format!(
            "output spec at index {output_index} requires `input` when multiple inputs are configured"
        ))),
    }
}

fn output_target_identity(output: &OutputSpec) -> Option<eng::FormatIdentity> {
    output.root_scope.format_identity_option()
}

fn run_convert_fanout<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: &cli::ConvertArgs,
    config: &RunConfig,
    positional_defaults: &ScopeConfig,
    s: &mut Streams<'_>,
) -> Outcome {
    if args.out.is_some() && config.outputs.len() > 1 {
        return handle_cli_request_error(
            CliRequestError::Usage(
                "convert with multiple --output-spec records requires per-output `dest`, not --out"
                    .to_owned(),
            ),
            s,
        );
    }

    let engine_context = context_with_config(&args.context, config);
    let inputs = match convert_configured_inputs(&engine_context, args, config, positional_defaults)
    {
        Ok(inputs) => inputs,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let mut report_inputs = Vec::new();
    let mut report_diagnostics = Vec::new();
    let mut report_scheduler_trace = cem_ml::report::SchedulerTraceReport::default();

    for (index, output) in config.outputs.iter().enumerate() {
        let destination = match output.destination.as_ref().map(PathBuf::from) {
            Some(destination) => destination,
            None if config.outputs.len() == 1 => {
                let input = match convert_input_for_output(output, index, &inputs) {
                    Ok(input) => input,
                    Err(err) => return handle_cli_request_error(err, s),
                };
                let input_uri = input.uri.clone();
                let req = eng::ConvertRequest {
                    input,
                    to_format: to_engine_layer_format(args.to_format),
                    preserve_source_offsets: args.preserve_source_offsets,
                    context: engine_context.clone(),
                    target: output_target_identity(output),
                    target_scope: output.root_scope.clone(),
                    scheduler_scope_id: index as u32,
                };
                match engine.convert(req) {
                    Ok(resp) => {
                        report_inputs.push(input_uri);
                        report_diagnostics.extend(resp.diagnostics.clone());
                        append_convert_scheduler_trace(
                            &mut report_scheduler_trace,
                            &resp.scheduler_trace,
                        );
                        if let Err(e) =
                            write_convert_primary(&engine_context, &resp, args.out.as_deref(), s)
                        {
                            let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                            return Outcome::code(EXIT_IO);
                        }
                        write_diagnostics(&resp.diagnostics, s);
                        if let Err(e) = write_convert_report_if_requested(
                            &engine_context,
                            args,
                            &report_inputs,
                            &report_diagnostics,
                            &report_scheduler_trace,
                        ) {
                            let _ = writeln!(s.stderr, "cem-ml: convert report write failure: {e}");
                            return Outcome::code(EXIT_IO);
                        }
                        return Outcome::ok();
                    }
                    Err(e) => return handle_engine_error(e, s),
                }
            }
            None => {
                return handle_cli_request_error(
                    CliRequestError::Usage(format!(
                        "output spec at index {index} requires `dest` for multi-output convert"
                    )),
                    s,
                );
            }
        };

        let input = match convert_input_for_output(output, index, &inputs) {
            Ok(input) => input,
            Err(err) => return handle_cli_request_error(err, s),
        };
        let input_uri = input.uri.clone();
        let req = eng::ConvertRequest {
            input,
            to_format: to_engine_layer_format(args.to_format),
            preserve_source_offsets: args.preserve_source_offsets,
            context: engine_context.clone(),
            target: output_target_identity(output),
            target_scope: output.root_scope.clone(),
            scheduler_scope_id: index as u32,
        };
        match engine.convert(req) {
            Ok(resp) => {
                report_inputs.push(input_uri);
                report_diagnostics.extend(resp.diagnostics.clone());
                append_convert_scheduler_trace(&mut report_scheduler_trace, &resp.scheduler_trace);
                if let Err(e) =
                    write_convert_primary(&engine_context, &resp, Some(destination.as_path()), s)
                {
                    let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                    return Outcome::code(EXIT_IO);
                }
                write_diagnostics(&resp.diagnostics, s);
            }
            Err(e) => return handle_engine_error(e, s),
        }
    }

    if let Err(e) = write_convert_report_if_requested(
        &engine_context,
        args,
        &report_inputs,
        &report_diagnostics,
        &report_scheduler_trace,
    ) {
        let _ = writeln!(s.stderr, "cem-ml: convert report write failure: {e}");
        return Outcome::code(EXIT_IO);
    }

    Outcome::ok()
}

fn handle_cli_request_error(err: CliRequestError, s: &mut Streams<'_>) -> Outcome {
    match err {
        CliRequestError::Usage(msg) => {
            let _ = writeln!(s.stderr, "cem-ml: {msg}");
            Outcome::code(EXIT_USAGE_OR_RESERVED)
        }
        CliRequestError::RunConfigDiagnostics { diagnostics, .. } => {
            let _ = writeln!(s.stderr, "cem-ml: invalid run config");
            write_diagnostics(&diagnostics, s);
            Outcome::code(EXIT_USAGE_OR_RESERVED)
        }
        CliRequestError::Engine(err) => handle_engine_error(err, s),
    }
}

fn handle_engine_error(err: EngineError, s: &mut Streams<'_>) -> Outcome {
    match err {
        EngineError::NotImplemented => {
            if !s.quiet {
                let _ = writeln!(
                    s.stderr,
                    "cem-ml: parser engine not yet implemented (see cem-ml-cli-plan.md Phase 11)."
                );
            }
            Outcome::ok()
        }
        EngineError::Io { .. } => {
            let _ = writeln!(s.stderr, "cem-ml: {err}");
            Outcome::code(EXIT_IO)
        }
        EngineError::SchemaResolution(_) => {
            let _ = writeln!(s.stderr, "cem-ml: {err}");
            Outcome::code(EXIT_SCHEMA)
        }
        EngineError::Internal(_) => {
            let _ = writeln!(s.stderr, "cem-ml: {err}");
            Outcome::code(EXIT_INTERNAL)
        }
        _ => {
            let _ = writeln!(s.stderr, "cem-ml: {err}");
            Outcome::code(EXIT_INTERNAL)
        }
    }
}

fn write_convert_primary(
    context: &eng::EngineContext,
    response: &eng::ConvertResponse,
    out: Option<&Path>,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    write_primary_with_bytes(
        context,
        &response.primary,
        response.primary_bytes.as_ref(),
        out,
        s,
    )
}

fn write_primary(
    context: &eng::EngineContext,
    primary: &serde_json::Value,
    out: Option<&Path>,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    write_primary_with_bytes(context, primary, None, out, s)
}

fn write_primary_with_bytes(
    context: &eng::EngineContext,
    primary: &serde_json::Value,
    primary_bytes: Option<&eng::PrimaryBytes>,
    out: Option<&Path>,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    if let Some(primary_bytes) = primary_bytes {
        return write_raw_primary_bytes(context, &primary_bytes.bytes, out, s);
    }

    if let Some(bytes) = binary_projection_primary_bytes(primary)? {
        return write_raw_primary_bytes(context, &bytes, out, s);
    }

    let serialized = serde_json::to_string_pretty(primary).unwrap_or_else(|_| String::new());
    match out {
        Some(path) => {
            write_destination(
                context,
                path,
                "output destination",
                ResolvePurpose::Output,
                serialized.as_bytes(),
            )?;
        }
        None => {
            writeln!(s.stdout, "{serialized}")?;
        }
    }
    Ok(())
}

fn write_raw_primary_bytes(
    context: &eng::EngineContext,
    bytes: &[u8],
    out: Option<&Path>,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    match out {
        Some(path) => write_destination(
            context,
            path,
            "output destination",
            ResolvePurpose::Output,
            bytes,
        )?,
        None => {
            s.stdout.write_all(bytes)?;
            s.stdout.flush()?;
        }
    }
    Ok(())
}

fn write_document_primary(
    context: &eng::EngineContext,
    primary: &serde_json::Value,
    out: Option<&Path>,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    let bytes;
    let body = if let Some(binary) = binary_projection_primary_bytes(primary)? {
        bytes = binary;
        &bytes
    } else if let Some(content) = document_primary_content(primary) {
        content.as_bytes()
    } else if let Some(projected) = collection_primary_output_value(primary) {
        bytes = serde_json::to_vec_pretty(&projected)?;
        &bytes
    } else {
        bytes = serde_json::to_vec_pretty(primary)?;
        &bytes
    };
    match out {
        Some(path) => write_destination(
            context,
            path,
            "output destination",
            ResolvePurpose::Output,
            body,
        )?,
        None => {
            s.stdout.write_all(body)?;
            s.stdout.flush()?;
        }
    }
    Ok(())
}

fn binary_projection_primary_bytes(primary: &serde_json::Value) -> io::Result<Option<Vec<u8>>> {
    if primary.get("kind").and_then(serde_json::Value::as_str) != Some("cem-binary-projection") {
        return Ok(None);
    }

    let Some(chunks) = primary.get("chunks") else {
        return Ok(None);
    };
    let chunks = chunks
        .as_array()
        .ok_or_else(|| invalid_binary_projection("chunks must be an array"))?;

    let mut decoded_chunks = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        if chunk.get("sealed").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err(invalid_binary_projection(
                "binary projection chunk is not sealed",
            ));
        }
        if chunk
            .get("dataEncoding")
            .and_then(serde_json::Value::as_str)
            != Some("hex")
        {
            return Err(invalid_binary_projection(
                "binary projection chunk must use hex dataEncoding",
            ));
        }
        let offset = chunk
            .get("byteOffset")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| invalid_binary_projection("missing chunk byteOffset"))?;
        let expected_len = chunk
            .get("byteLength")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| invalid_binary_projection("missing chunk byteLength"))?;
        let data = chunk
            .get("data")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_binary_projection("missing chunk data"))?;
        let bytes = decode_hex_chunk(data)?;
        if bytes.len() as u64 != expected_len {
            return Err(invalid_binary_projection(
                "chunk byteLength does not match decoded data",
            ));
        }
        decoded_chunks.push((offset, bytes));
    }

    decoded_chunks.sort_by_key(|(offset, _)| *offset);
    let mut out = Vec::new();
    for (offset, chunk_bytes) in decoded_chunks {
        if offset != out.len() as u64 {
            return Err(invalid_binary_projection(
                "binary projection chunks are not contiguous",
            ));
        }
        out.extend_from_slice(&chunk_bytes);
    }

    if let Some(expected_len) = primary
        .get("byteLength")
        .and_then(serde_json::Value::as_u64)
    {
        if out.len() as u64 != expected_len {
            return Err(invalid_binary_projection(
                "artifact byteLength does not match decoded chunks",
            ));
        }
    }

    Ok(Some(out))
}

fn decode_hex_chunk(data: &str) -> io::Result<Vec<u8>> {
    if data.len() % 2 != 0 {
        return Err(invalid_binary_projection("hex chunk has odd length"));
    }

    let mut out = Vec::with_capacity(data.len() / 2);
    for pair in data.as_bytes().chunks_exact(2) {
        let high = hex_nibble(pair[0])
            .ok_or_else(|| invalid_binary_projection("hex chunk contains invalid digit"))?;
        let low = hex_nibble(pair[1])
            .ok_or_else(|| invalid_binary_projection("hex chunk contains invalid digit"))?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn invalid_binary_projection(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn collection_primary_output_value(primary: &serde_json::Value) -> Option<serde_json::Value> {
    if primary.get("kind").and_then(serde_json::Value::as_str) != Some("collection") {
        return None;
    }
    let items = primary
        .get("items")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .map(|item| {
            serde_json::json!({
                "input": item.get("input").cloned().unwrap_or(serde_json::Value::Null),
                "artifactId": item.get("artifactId").cloned().unwrap_or(serde_json::Value::Null),
                "uri": item.get("uri").cloned().unwrap_or(serde_json::Value::Null),
                "identity": item.get("identity").cloned().unwrap_or(serde_json::Value::Null),
                "primary": item.get("primary").cloned().unwrap_or(serde_json::Value::Null),
                "bindings": item.get("bindings").cloned().unwrap_or_else(|| serde_json::json!({})),
            })
        })
        .collect::<Vec<_>>();

    Some(serde_json::json!({
        "kind": "collection",
        "mode": primary.get("mode").cloned().unwrap_or(serde_json::Value::Null),
        "count": primary.get("count").cloned().unwrap_or_else(|| serde_json::json!(items.len())),
        "bindings": primary.get("bindings").cloned().unwrap_or_else(|| serde_json::json!({})),
        "items": items,
    }))
}

fn document_primary_content(primary: &serde_json::Value) -> Option<&str> {
    match primary {
        serde_json::Value::String(value) => Some(value.as_str()),
        serde_json::Value::Object(fields) => {
            fields.get("content").and_then(serde_json::Value::as_str)
        }
        _ => None,
    }
}

fn write_destination(
    context: &eng::EngineContext,
    path: &Path,
    label: &str,
    purpose: ResolvePurpose,
    contents: &[u8],
) -> io::Result<()> {
    let raw = path.to_string_lossy();
    if raw.starts_with("file://") {
        if let Some(path) = local_file_uri_to_path(&raw) {
            return write_local_destination(&path, contents);
        }
        if write_registered_destination(context, &raw, purpose, contents)? {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported {label} `{raw}`; only local file:// URIs are supported"),
        ));
    }

    if uri_scheme(&raw).is_some() && !is_windows_drive_path(&raw) {
        if write_registered_destination(context, &raw, purpose, contents)? {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported {label} `{raw}`; remote/custom URI resolvers are not implemented"),
        ));
    }

    write_local_destination(path, contents)
}

fn write_local_destination(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, contents)
}

fn write_registered_destination(
    context: &eng::EngineContext,
    uri: &str,
    purpose: ResolvePurpose,
    contents: &[u8],
) -> io::Result<bool> {
    let request = ResolveRequest::new(uri, purpose, ResolveDirection::Write);
    match context.resolver_registry.write(&request, contents) {
        Ok(_) => Ok(true),
        Err(ResolverDiagnostic::UnsupportedResolver { .. }) => Ok(false),
        Err(error) => Err(io::Error::other(error)),
    }
}

/// Tokenize each input and run the cem-ql template embedding pass
/// (AC-T-7). Returns the cem-ql diagnostics that must be merged into
/// the engine's report. HTML / XML inputs short-circuit to empty.
fn collect_embedding_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let from = input.from_format.unwrap_or(eng::InputFormat::Cem);
        diagnostics.extend(template_pass::run(&input.bytes, from, Some(&input.uri)));
    }
    diagnostics
}

fn direct_source_validation_report(
    inputs: &[eng::EngineInput],
    fail_level: cli::FailLevel,
    context: &cli::ContextOptions,
) -> Option<cem_ml::report::Report> {
    if inputs.is_empty() {
        return None;
    }

    let mut diagnostics = Vec::new();
    for input in inputs {
        if is_cem_ql_source_input(input) {
            diagnostics.extend(collect_cem_ql_source_diagnostics(std::slice::from_ref(
                input,
            )));
        } else if is_cem_dom_projection_source_input(input) {
            diagnostics.extend(collect_cem_dom_projection_source_diagnostics(
                std::slice::from_ref(input),
            ));
        } else if is_cem_ast_projection_source_input(input) {
            diagnostics.extend(collect_cem_ast_projection_source_diagnostics(
                std::slice::from_ref(input),
            ));
        } else if is_cem_events_projection_source_input(input) {
            diagnostics.extend(collect_cem_events_projection_source_diagnostics(
                std::slice::from_ref(input),
            ));
        } else if is_json_schema_source_input(input) {
            diagnostics.extend(collect_json_schema_source_diagnostics(
                std::slice::from_ref(input),
            ));
        } else if is_yaml_source_input(input) {
            diagnostics.extend(collect_yaml_source_diagnostics(std::slice::from_ref(input)));
        } else if is_csv_source_input(input) {
            diagnostics.extend(collect_csv_source_diagnostics(std::slice::from_ref(input)));
        } else if is_markdown_source_input(input) {
            diagnostics.extend(collect_markdown_source_diagnostics(std::slice::from_ref(
                input,
            )));
        } else if is_relax_ng_source_input(input) {
            diagnostics.extend(collect_relax_ng_source_diagnostics(std::slice::from_ref(
                input,
            )));
        } else if is_xml_source_input(input) {
            diagnostics.extend(collect_xml_source_diagnostics(std::slice::from_ref(input)));
        } else if is_json_source_input(input) {
            diagnostics.extend(collect_json_source_diagnostics(std::slice::from_ref(input)));
        } else {
            return None;
        }
    }

    Some(cem_ml::report::Report::deterministic(
        inputs.iter().map(|input| input.uri.clone()).collect(),
        diagnostics,
        report_options_snapshot(fail_level, context),
    ))
}

fn is_cem_ql_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_cem_ql = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::CEM_QL_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_cem_ql_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_cem_ql
        }
        Some(_) => false,
        None => schema_is_cem_ql,
    }
}

fn is_cem_ql_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::CEM_QL_CONTENT_TYPE | "text/cem-ql"
    )
}

fn is_json_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_json = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_json_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_json
        }
        Some(_) => false,
        None => schema_is_json,
    }
}

fn is_json_schema_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_json_schema = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::JSON_SCHEMA_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type)
            if content_type == cem_ml::schema::registry::JSON_SCHEMA_CONTENT_TYPE =>
        {
            identity.schema.is_none() || schema_is_json_schema
        }
        Some(_) => false,
        None => schema_is_json_schema,
    }
}

fn is_yaml_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_yaml = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::YAML_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_yaml_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_yaml
        }
        Some(_) => false,
        None => schema_is_yaml,
    }
}

fn is_csv_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_csv = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::CSV_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_csv_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_csv
        }
        Some(_) => false,
        None => schema_is_csv,
    }
}

fn is_markdown_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_markdown = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::MARKDOWN_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_markdown_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_markdown
        }
        Some(_) => false,
        None => schema_is_markdown,
    }
}

fn is_xml_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_xml = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::XML_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_xml_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_xml
        }
        Some(_) => false,
        None => schema_is_xml,
    }
}

fn is_relax_ng_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_relax_ng = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::RELAX_NG_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_relax_ng_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_relax_ng
        }
        Some(_) => false,
        None => schema_is_relax_ng,
    }
}

fn is_cem_dom_projection_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_cem_dom = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(is_cem_dom_projection_schema);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_cem_dom_projection_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_cem_dom
        }
        Some(_) => false,
        None => schema_is_cem_dom,
    }
}

fn is_cem_dom_projection_schema(schema: &str) -> bool {
    matches!(
        schema,
        cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI
            | cem_ml::lifecycle::DOM_JSON_PROJECTION_SCHEMA
    )
}

fn is_cem_dom_projection_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE
            | cem_ml::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE
    )
}

fn is_cem_ast_projection_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_cem_ast = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_cem_ast_projection_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_cem_ast
        }
        Some(_) => false,
        None => schema_is_cem_ast,
    }
}

fn is_cem_ast_projection_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::CEM_AST_PROJECTION_CONTENT_TYPE
            | cem_ml::schema::registry::CEM_AST_JSON_PROJECTION_CONTENT_TYPE
    )
}

fn is_cem_events_projection_source_input(input: &eng::EngineInput) -> bool {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let schema_is_cem_events = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI);
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type) if is_cem_events_projection_source_content_type(content_type) => {
            identity.schema.is_none() || schema_is_cem_events
        }
        Some(_) => false,
        None => schema_is_cem_events,
    }
}

fn is_cem_events_projection_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::CEM_EVENTS_PROJECTION_CONTENT_TYPE
            | cem_ml::schema::registry::CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE
    )
}

fn is_json_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::JSON_CONTENT_TYPE | "text/json"
    )
}

fn is_yaml_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::YAML_CONTENT_TYPE
            | "application/x-yaml"
            | "text/yaml"
            | "text/x-yaml"
    )
}

fn is_csv_source_content_type(content_type: &str) -> bool {
    content_type == cem_ml::schema::registry::CSV_CONTENT_TYPE
}

fn is_markdown_source_content_type(content_type: &str) -> bool {
    content_type == cem_ml::schema::registry::MARKDOWN_CONTENT_TYPE
}

fn is_relax_ng_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::RELAX_NG_XML_CONTENT_TYPE
            | cem_ml::schema::registry::RELAX_NG_COMPACT_CONTENT_TYPE
    )
}

fn is_xml_source_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        cem_ml::schema::registry::XML_CONTENT_TYPE
            | "text/xml"
            | "application/xml-external-parsed-entity"
            | "text/xml-external-parsed-entity"
            | "application/xml-dtd"
    )
}

fn cli_content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn cli_content_type_parameter(content_type: &str, name: &str) -> Option<String> {
    let needle = name.trim().to_ascii_lowercase();
    content_type.split(';').skip(1).find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key.trim().eq_ignore_ascii_case(&needle) {
            Some(value.trim().trim_matches('"').to_owned())
        } else {
            None
        }
    })
}

fn input_source_content_type(input: &eng::EngineInput) -> Option<String> {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    identity.content_type
}

fn collect_cem_ql_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let Ok(source) = std::str::from_utf8(&input.bytes) else {
            diagnostics.push(cem_ql_invalid_utf8_diagnostic(input));
            continue;
        };

        let parsed = cem_ql::api::parse(source);
        if !parsed.module.nodes.iter().any(|node| {
            matches!(
                node,
                cem_ql::parser::SurfaceNode::Module(module) if !module.uri.trim().is_empty()
            )
        }) {
            diagnostics.push(cem_ql_module_uri_missing_diagnostic(input));
        }
        diagnostics.extend(parsed.diagnostics.into_iter().map(|mut diagnostic| {
            diagnostic.uri = Some(input.uri.clone());
            diagnostic
        }));
    }
    diagnostics
}

fn cem_ql_invalid_utf8_diagnostic(input: &eng::EngineInput) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        code: "cem.ql.invalid_utf8".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: "CEM-QL source must be valid UTF-8".to_owned(),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn cem_ql_module_uri_missing_diagnostic(
    input: &eng::EngineInput,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        code: "cem.ql.module_uri_missing".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: "CEM-QL module source requires a `module \"...\"` URI declaration".to_owned(),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn collect_json_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        if let Err(error) = serde_json::from_slice::<serde_json::Value>(&input.bytes) {
            diagnostics.push(json_parse_error_diagnostic(input, &error));
        }
    }
    diagnostics
}

fn collect_json_schema_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let value = match serde_json::from_slice::<serde_json::Value>(&input.bytes) {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(json_schema_parse_error_diagnostic(input, &error));
                continue;
            }
        };
        if let Some(diagnostic) = json_schema_dialect_diagnostic(input, &value) {
            diagnostics.push(diagnostic);
        }
    }
    diagnostics
}

fn collect_yaml_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let source = match std::str::from_utf8(&input.bytes) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(yaml_unsupported_encoding_diagnostic(input, &error));
                continue;
            }
        };

        let mut receiver = YamlValidationReceiver {
            input,
            diagnostics: Vec::new(),
        };
        let mut parser = yaml_rust2::parser::Parser::new_from_str(source);
        if let Err(error) = parser.load(&mut receiver, true) {
            diagnostics.push(yaml_parse_error_diagnostic(input, &error));
        }
        diagnostics.extend(receiver.diagnostics);
    }
    diagnostics
}

fn collect_csv_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let source = match std::str::from_utf8(&input.bytes) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(csv_unsupported_encoding_diagnostic(input, &error));
                continue;
            }
        };

        diagnostics.extend(collect_csv_quote_policy_diagnostics(input, source));

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(source.as_bytes());
        let mut expected_field_count = None;
        for (row_index, result) in reader.records().enumerate() {
            match result {
                Ok(record) => {
                    let field_count = record.len();
                    if let Some(expected) = expected_field_count {
                        if field_count != expected {
                            diagnostics.push(csv_inconsistent_field_count_diagnostic(
                                input,
                                record.position(),
                                row_index + 1,
                                expected,
                                field_count,
                            ));
                        }
                    } else {
                        expected_field_count = Some(field_count);
                    }
                }
                Err(error) => {
                    diagnostics.push(csv_parse_error_diagnostic(input, &error));
                    break;
                }
            }
        }
    }
    diagnostics
}

fn collect_markdown_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let content_type = input_source_content_type(input);
        if content_type
            .as_deref()
            .is_some_and(markdown_content_type_missing_charset)
        {
            diagnostics.push(markdown_charset_missing_diagnostic(input));
        }
        let variant = content_type
            .as_deref()
            .and_then(|content_type| cli_content_type_parameter(content_type, "variant"));
        if let Some(variant) = variant.as_deref() {
            if !markdown_variant_is_known(variant) {
                diagnostics.push(markdown_unknown_variant_diagnostic(input, variant));
            }
        }

        let source = match std::str::from_utf8(&input.bytes) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(markdown_unsupported_encoding_diagnostic(input, &error));
                continue;
            }
        };

        let options = markdown_parser_options(variant.as_deref());
        let parser = pulldown_cmark::Parser::new_ext(source, options).into_offset_iter();
        for (event, range) in parser {
            if matches!(
                event,
                pulldown_cmark::Event::Html(_) | pulldown_cmark::Event::InlineHtml(_)
            ) {
                diagnostics.push(markdown_embedded_html_rejected_diagnostic(
                    input,
                    source,
                    range.start,
                ));
            }
        }
    }
    diagnostics
}

fn collect_relax_ng_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let source = match std::str::from_utf8(&input.bytes) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(relax_ng_unsupported_encoding_diagnostic(input, &error));
                continue;
            }
        };

        match relax_ng_source_kind(input) {
            RelaxNgSourceKind::Xml => {
                diagnostics.extend(validate_relax_ng_xml_source(input, source));
            }
            RelaxNgSourceKind::Compact => {
                diagnostics.extend(validate_relax_ng_compact_source(input, source));
            }
        }
    }
    diagnostics
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelaxNgSourceKind {
    Xml,
    Compact,
}

fn relax_ng_source_kind(input: &eng::EngineInput) -> RelaxNgSourceKind {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type)
            if content_type == cem_ml::schema::registry::RELAX_NG_COMPACT_CONTENT_TYPE =>
        {
            RelaxNgSourceKind::Compact
        }
        _ => RelaxNgSourceKind::Xml,
    }
}

#[derive(Clone, Debug)]
struct RelaxNgXmlFrame {
    local_name: String,
    namespace_uri: String,
    missing_name_attribute: bool,
    has_name_class_child: bool,
}

fn validate_relax_ng_xml_source(
    input: &eng::EngineInput,
    source: &str,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;

    let mut element_stack: Vec<RelaxNgXmlFrame> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut saw_grammar_root = false;
    let mut saw_start = false;

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                root_count += usize::from(element_stack.is_empty());
                let start_offset = xml_event_position(&reader, &start, false);
                let is_root = element_stack.is_empty();
                let (frame, namespaces, mut event_diagnostics) = relax_ng_xml_start_frame(
                    input,
                    source,
                    &start,
                    &namespace_stack,
                    start_offset,
                    is_root,
                );
                diagnostics.append(&mut event_diagnostics);
                relax_ng_xml_note_child_name_class(&mut element_stack, &frame);
                if element_stack.is_empty() {
                    if root_count > 1 {
                        diagnostics.push(relax_ng_diagnostic(
                            input,
                            source,
                            start_offset,
                            "cem.relax_ng.xml_parse_error",
                            cem_ml::diagnostics::Severity::Error,
                            "RELAX NG XML syntax must have exactly one document element".to_owned(),
                        ));
                    }
                    saw_grammar_root = frame.namespace_uri
                        == cem_ml::schema::registry::RELAX_NG_NAMESPACE_URI
                        && frame.local_name == "grammar";
                }
                if frame.namespace_uri == cem_ml::schema::registry::RELAX_NG_NAMESPACE_URI
                    && frame.local_name == "start"
                {
                    saw_start = true;
                }
                element_stack.push(frame);
                namespace_stack.push(namespaces);
            }
            Ok(quick_xml::events::Event::Empty(start)) => {
                root_count += usize::from(element_stack.is_empty());
                let start_offset = xml_event_position(&reader, &start, true);
                let is_root = element_stack.is_empty();
                let (frame, _, mut event_diagnostics) = relax_ng_xml_start_frame(
                    input,
                    source,
                    &start,
                    &namespace_stack,
                    start_offset,
                    is_root,
                );
                diagnostics.append(&mut event_diagnostics);
                relax_ng_xml_note_child_name_class(&mut element_stack, &frame);
                if element_stack.is_empty() {
                    if root_count > 1 {
                        diagnostics.push(relax_ng_diagnostic(
                            input,
                            source,
                            start_offset,
                            "cem.relax_ng.xml_parse_error",
                            cem_ml::diagnostics::Severity::Error,
                            "RELAX NG XML syntax must have exactly one document element".to_owned(),
                        ));
                    }
                    saw_grammar_root = frame.namespace_uri
                        == cem_ml::schema::registry::RELAX_NG_NAMESPACE_URI
                        && frame.local_name == "grammar";
                }
                if frame.namespace_uri == cem_ml::schema::registry::RELAX_NG_NAMESPACE_URI
                    && frame.local_name == "start"
                {
                    saw_start = true;
                }
                diagnostics.extend(relax_ng_xml_close_frame(input, source, frame, start_offset));
            }
            Ok(quick_xml::events::Event::End(_)) => {
                if let Some(frame) = element_stack.pop() {
                    diagnostics.extend(relax_ng_xml_close_frame(
                        input,
                        source,
                        frame,
                        Some(reader.error_position()),
                    ));
                    if namespace_stack.len() > 1 {
                        namespace_stack.pop();
                    }
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => diagnostics.push(relax_ng_diagnostic(
                input,
                source,
                Some(reader.error_position()),
                "cem.relax_ng.xml_parse_error",
                cem_ml::diagnostics::Severity::Error,
                "RELAX NG XML syntax must not contain a DTD".to_owned(),
            )),
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(relax_ng_xml_parse_error_diagnostic(
                    input,
                    source,
                    Some(reader.error_position()),
                    &error,
                ));
                break;
            }
        }
    }

    if root_count == 0 {
        diagnostics.push(relax_ng_diagnostic(
            input,
            source,
            Some(0),
            "cem.relax_ng.xml_parse_error",
            cem_ml::diagnostics::Severity::Error,
            "RELAX NG XML syntax must contain a document element".to_owned(),
        ));
    }
    if saw_grammar_root && !saw_start {
        diagnostics.push(relax_ng_diagnostic(
            input,
            source,
            Some(0),
            "cem.relax_ng.missing_start",
            cem_ml::diagnostics::Severity::Error,
            "RELAX NG grammar must declare a start pattern".to_owned(),
        ));
    }

    diagnostics
}

fn relax_ng_xml_start_frame(
    input: &eng::EngineInput,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
    byte_offset: Option<u64>,
    is_root: bool,
) -> (
    RelaxNgXmlFrame,
    BTreeMap<String, String>,
    Vec<cem_ml::diagnostics::Diagnostic>,
) {
    let mut diagnostics = Vec::new();
    let mut namespaces = namespace_stack
        .last()
        .cloned()
        .unwrap_or_else(xml_initial_namespaces);
    let mut attributes = BTreeSet::new();

    for attribute in start.attributes().with_checks(false) {
        match attribute {
            Ok(attribute) => {
                let name = xml_qname_display(attribute.key.as_ref());
                let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                if name == "xmlns" {
                    namespaces.insert(String::new(), value);
                } else if let Some(prefix) = name.strip_prefix("xmlns:") {
                    namespaces.insert(prefix.to_owned(), value);
                } else {
                    attributes.insert(name);
                }
            }
            Err(error) => diagnostics.push(relax_ng_diagnostic(
                input,
                source,
                byte_offset,
                "cem.relax_ng.xml_parse_error",
                cem_ml::diagnostics::Severity::Error,
                format!("RELAX NG XML attribute parse error: {error}"),
            )),
        }
    }

    let qualified_name = xml_qname_display(start.name().as_ref());
    let (namespace_uri, local_name) = relax_ng_xml_expanded_name(&qualified_name, &namespaces);
    let is_rng = namespace_uri == cem_ml::schema::registry::RELAX_NG_NAMESPACE_URI;
    let mut missing_name_attribute = false;

    if !is_rng && is_root {
        diagnostics.push(relax_ng_diagnostic(
            input,
            source,
            byte_offset,
            "cem.relax_ng.unknown_element",
            cem_ml::diagnostics::Severity::Error,
            format!(
                "RELAX NG XML root element `{qualified_name}` is not in the RELAX NG structure namespace"
            ),
        ));
    } else if is_rng && !relax_ng_xml_element_is_known(&local_name) {
        diagnostics.push(relax_ng_diagnostic(
            input,
            source,
            byte_offset,
            "cem.relax_ng.unknown_element",
            cem_ml::diagnostics::Severity::Error,
            format!("unknown RELAX NG XML element `{local_name}`"),
        ));
    } else if is_rng && matches!(local_name.as_str(), "include" | "externalRef") {
        diagnostics.push(relax_ng_diagnostic(
            input,
            source,
            byte_offset,
            if local_name == "include" {
                "cem.relax_ng.include_rejected"
            } else {
                "cem.relax_ng.external_ref_rejected"
            },
            cem_ml::diagnostics::Severity::Error,
            format!("RELAX NG `{local_name}` is rejected until resolver policy enables it"),
        ));
    }

    if is_rng {
        match local_name.as_str() {
            "define" | "ref" | "parentRef" if !attributes.contains("name") => {
                diagnostics.push(relax_ng_missing_attribute_diagnostic(
                    input,
                    source,
                    byte_offset,
                    &local_name,
                    "name",
                ));
            }
            "data" if !attributes.contains("type") => diagnostics.push(
                relax_ng_missing_attribute_diagnostic(input, source, byte_offset, "data", "type"),
            ),
            "include" | "externalRef" if !attributes.contains("href") => {
                diagnostics.push(relax_ng_missing_attribute_diagnostic(
                    input,
                    source,
                    byte_offset,
                    &local_name,
                    "href",
                ));
            }
            "element" | "attribute" if !attributes.contains("name") => {
                missing_name_attribute = true;
            }
            _ => {}
        }
    }

    (
        RelaxNgXmlFrame {
            local_name,
            namespace_uri,
            missing_name_attribute,
            has_name_class_child: false,
        },
        namespaces,
        diagnostics,
    )
}

fn relax_ng_xml_close_frame(
    input: &eng::EngineInput,
    source: &str,
    frame: RelaxNgXmlFrame,
    byte_offset: Option<u64>,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    if frame.missing_name_attribute && !frame.has_name_class_child {
        return vec![relax_ng_missing_attribute_diagnostic(
            input,
            source,
            byte_offset,
            &frame.local_name,
            "name",
        )];
    }
    Vec::new()
}

fn relax_ng_xml_note_child_name_class(stack: &mut [RelaxNgXmlFrame], child: &RelaxNgXmlFrame) {
    let Some(parent) = stack.last_mut() else {
        return;
    };
    if parent.namespace_uri != cem_ml::schema::registry::RELAX_NG_NAMESPACE_URI
        || !matches!(parent.local_name.as_str(), "element" | "attribute")
    {
        return;
    }
    if child.namespace_uri == cem_ml::schema::registry::RELAX_NG_NAMESPACE_URI
        && matches!(
            child.local_name.as_str(),
            "name" | "anyName" | "nsName" | "choice"
        )
    {
        parent.has_name_class_child = true;
    }
}

fn relax_ng_xml_expanded_name(
    qualified_name: &str,
    namespaces: &BTreeMap<String, String>,
) -> (String, String) {
    if let Some((prefix, local_name)) = qualified_name.split_once(':') {
        (
            namespaces.get(prefix).cloned().unwrap_or_default(),
            local_name.to_owned(),
        )
    } else {
        (
            namespaces.get("").cloned().unwrap_or_default(),
            qualified_name.to_owned(),
        )
    }
}

fn relax_ng_xml_element_is_known(local_name: &str) -> bool {
    matches!(
        local_name,
        "grammar"
            | "start"
            | "define"
            | "div"
            | "include"
            | "element"
            | "attribute"
            | "choice"
            | "group"
            | "interleave"
            | "oneOrMore"
            | "zeroOrMore"
            | "optional"
            | "list"
            | "mixed"
            | "ref"
            | "parentRef"
            | "empty"
            | "text"
            | "value"
            | "data"
            | "param"
            | "except"
            | "notAllowed"
            | "externalRef"
            | "name"
            | "anyName"
            | "nsName"
    )
}

fn validate_relax_ng_compact_source(
    input: &eng::EngineInput,
    source: &str,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut code = String::with_capacity(source.len());
    let mut chars = source.char_indices().peekable();
    let mut has_start = false;

    while let Some((offset, ch)) = chars.next() {
        match ch {
            '#' => {
                code.push(' ');
                while let Some((_, comment_ch)) = chars.peek().copied() {
                    if comment_ch == '\n' {
                        break;
                    }
                    chars.next();
                    code.push(' ');
                }
            }
            '"' | '\'' => {
                code.push(' ');
                let quote = ch;
                let mut escaped = false;
                let mut closed = false;
                for (_, string_ch) in chars.by_ref() {
                    code.push(if string_ch == '\n' { '\n' } else { ' ' });
                    if escaped {
                        escaped = false;
                    } else if string_ch == '\\' {
                        escaped = true;
                    } else if string_ch == quote {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    diagnostics.push(relax_ng_diagnostic(
                        input,
                        source,
                        Some(offset as u64),
                        "cem.relax_ng.compact_parse_error",
                        cem_ml::diagnostics::Severity::Error,
                        "RELAX NG compact string literal is missing a closing quote".to_owned(),
                    ));
                }
            }
            '{' | '(' | '[' => {
                stack.push((ch, offset));
                code.push(ch);
            }
            '}' | ')' | ']' => {
                let expected = match ch {
                    '}' => '{',
                    ')' => '(',
                    ']' => '[',
                    _ => unreachable!(),
                };
                match stack.pop() {
                    Some((open, _)) if open == expected => {}
                    Some((open, open_offset)) => diagnostics.push(relax_ng_diagnostic(
                        input,
                        source,
                        Some(offset as u64),
                        "cem.relax_ng.compact_parse_error",
                        cem_ml::diagnostics::Severity::Error,
                        format!(
                            "RELAX NG compact closing delimiter `{ch}` does not match `{open}` opened at byte {open_offset}"
                        ),
                    )),
                    None => diagnostics.push(relax_ng_diagnostic(
                        input,
                        source,
                        Some(offset as u64),
                        "cem.relax_ng.compact_parse_error",
                        cem_ml::diagnostics::Severity::Error,
                        format!("RELAX NG compact closing delimiter `{ch}` has no opening delimiter"),
                    )),
                }
                code.push(ch);
            }
            _ => code.push(ch),
        }
    }

    for (open, offset) in stack {
        diagnostics.push(relax_ng_diagnostic(
            input,
            source,
            Some(offset as u64),
            "cem.relax_ng.compact_parse_error",
            cem_ml::diagnostics::Severity::Error,
            format!("RELAX NG compact delimiter `{open}` is not closed"),
        ));
    }

    for token in relax_ng_compact_tokens(&code) {
        match token.as_str() {
            "start" => has_start = true,
            "include" => diagnostics.push(relax_ng_diagnostic(
                input,
                source,
                None,
                "cem.relax_ng.include_rejected",
                cem_ml::diagnostics::Severity::Error,
                "RELAX NG compact include is rejected until resolver policy enables it".to_owned(),
            )),
            "external" => diagnostics.push(relax_ng_diagnostic(
                input,
                source,
                None,
                "cem.relax_ng.external_ref_rejected",
                cem_ml::diagnostics::Severity::Error,
                "RELAX NG compact external reference is rejected until resolver policy enables it"
                    .to_owned(),
            )),
            _ => {}
        }
    }

    if !has_start {
        diagnostics.push(relax_ng_diagnostic(
            input,
            source,
            Some(0),
            "cem.relax_ng.missing_start",
            cem_ml::diagnostics::Severity::Error,
            "RELAX NG compact syntax must declare a start pattern".to_owned(),
        ));
    }

    diagnostics
}

fn relax_ng_compact_tokens(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in source.chars() {
        if ch == '_' || ch == '-' || ch.is_ascii_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn relax_ng_missing_attribute_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: Option<u64>,
    element: &str,
    attribute: &str,
) -> cem_ml::diagnostics::Diagnostic {
    relax_ng_diagnostic(
        input,
        source,
        byte_offset,
        "cem.relax_ng.missing_required_attribute",
        cem_ml::diagnostics::Severity::Error,
        format!("RELAX NG `{element}` requires `{attribute}`"),
    )
}

fn relax_ng_xml_parse_error_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: Option<u64>,
    error: &quick_xml::Error,
) -> cem_ml::diagnostics::Diagnostic {
    relax_ng_diagnostic(
        input,
        source,
        byte_offset,
        match error {
            quick_xml::Error::Encoding(_) => "cem.relax_ng.unsupported_encoding",
            _ => "cem.relax_ng.xml_parse_error",
        },
        cem_ml::diagnostics::Severity::Error,
        format!("RELAX NG XML parse error: {error}"),
    )
}

fn relax_ng_unsupported_encoding_diagnostic(
    input: &eng::EngineInput,
    error: &std::str::Utf8Error,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        byte_offset: u64::try_from(error.valid_up_to()).ok(),
        code: "cem.relax_ng.unsupported_encoding".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("RELAX NG source must be valid UTF-8: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn relax_ng_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: Option<u64>,
    code: &'static str,
    severity: cem_ml::diagnostics::Severity,
    message: String,
) -> cem_ml::diagnostics::Diagnostic {
    let (line, column) = byte_offset
        .and_then(|offset| usize::try_from(offset).ok())
        .map(|offset| markdown_line_col(source, offset))
        .map(|(line, column)| (Some(line), Some(column)))
        .unwrap_or((None, None));
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line,
        column,
        byte_offset,
        code: code.to_owned(),
        severity,
        message,
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn collect_xml_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let source = match std::str::from_utf8(&input.bytes) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(xml_unsupported_utf8_diagnostic(input, &error));
                continue;
            }
        };

        let content_type = input_source_content_type(input);
        let mime_charset = content_type
            .as_deref()
            .and_then(|content_type| cli_content_type_parameter(content_type, "charset"));
        if let Some(charset) = mime_charset.as_deref() {
            if !xml_encoding_is_supported(charset) {
                diagnostics.push(xml_unsupported_encoding_diagnostic(
                    input,
                    source,
                    None,
                    &format!("XML content-type charset `{charset}` is not supported"),
                ));
                continue;
            }
        }

        match xml_source_kind(input) {
            XmlSourceKind::Dtd => {
                if !source.trim().is_empty() {
                    diagnostics.push(xml_diagnostic(
                        input,
                        source,
                        None,
                        "cem.xml.dtd_rejected",
                        cem_ml::diagnostics::Severity::Error,
                        "XML DTD resources are rejected until an explicit DTD policy enables them"
                            .to_owned(),
                    ));
                }
            }
            XmlSourceKind::Document | XmlSourceKind::ExternalParsedEntity => {
                diagnostics.extend(validate_xml_source(input, source, mime_charset.as_deref()));
            }
        }
    }
    diagnostics
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum XmlSourceKind {
    Document,
    ExternalParsedEntity,
    Dtd,
}

fn xml_source_kind(input: &eng::EngineInput) -> XmlSourceKind {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some("application/xml-dtd") => XmlSourceKind::Dtd,
        Some("application/xml-external-parsed-entity")
        | Some("text/xml-external-parsed-entity") => XmlSourceKind::ExternalParsedEntity,
        _ => XmlSourceKind::Document,
    }
}

fn validate_xml_source(
    input: &eng::EngineInput,
    source: &str,
    mime_charset: Option<&str>,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let kind = xml_source_kind(input);
    let mut diagnostics = Vec::new();
    let mut reader = quick_xml::Reader::from_str(source);
    reader.config_mut().check_comments = true;

    let mut element_stack: Vec<String> = Vec::new();
    let mut namespace_stack = vec![xml_initial_namespaces()];
    let mut root_count = 0usize;
    let mut reported_multiple_roots = false;

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(start)) => {
                if element_stack.is_empty() {
                    root_count += 1;
                    if kind == XmlSourceKind::Document && root_count > 1 && !reported_multiple_roots
                    {
                        diagnostics.push(xml_diagnostic(
                            input,
                            source,
                            xml_event_position(&reader, &start, false),
                            "cem.xml.parse_error",
                            cem_ml::diagnostics::Severity::Error,
                            "XML document must have exactly one document element".to_owned(),
                        ));
                        reported_multiple_roots = true;
                    }
                }

                let (next_namespaces, mut start_diagnostics) =
                    validate_xml_start_event(input, source, &start, &namespace_stack);
                diagnostics.append(&mut start_diagnostics);
                element_stack.push(xml_qname_display(start.name().as_ref()));
                namespace_stack.push(next_namespaces);
            }
            Ok(quick_xml::events::Event::Empty(start)) => {
                if element_stack.is_empty() {
                    root_count += 1;
                    if kind == XmlSourceKind::Document && root_count > 1 && !reported_multiple_roots
                    {
                        diagnostics.push(xml_diagnostic(
                            input,
                            source,
                            xml_event_position(&reader, &start, true),
                            "cem.xml.parse_error",
                            cem_ml::diagnostics::Severity::Error,
                            "XML document must have exactly one document element".to_owned(),
                        ));
                        reported_multiple_roots = true;
                    }
                }

                let (_, mut start_diagnostics) =
                    validate_xml_start_event(input, source, &start, &namespace_stack);
                diagnostics.append(&mut start_diagnostics);
            }
            Ok(quick_xml::events::Event::End(end)) => {
                let found = xml_qname_display(end.name().as_ref());
                match element_stack.pop() {
                    Some(expected) if expected == found => {
                        if namespace_stack.len() > 1 {
                            namespace_stack.pop();
                        }
                    }
                    Some(expected) => diagnostics.push(xml_diagnostic(
                        input,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        cem_ml::diagnostics::Severity::Error,
                        format!("XML end tag `</{found}>` does not match `<{expected}>`"),
                    )),
                    None => diagnostics.push(xml_diagnostic(
                        input,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        cem_ml::diagnostics::Severity::Error,
                        format!("XML end tag `</{found}>` has no matching start tag"),
                    )),
                }
            }
            Ok(quick_xml::events::Event::Decl(decl)) => {
                if let Err(error) = decl.version() {
                    diagnostics.push(xml_reader_error_diagnostic(
                        input,
                        source,
                        Some(reader.error_position()),
                        &error,
                    ));
                }
                if let Some(encoding) = decl.encoding() {
                    match encoding {
                        Ok(encoding) => {
                            let encoding = String::from_utf8_lossy(encoding.as_ref());
                            if !xml_encoding_is_supported(&encoding) {
                                diagnostics.push(xml_unsupported_encoding_diagnostic(
                                    input,
                                    source,
                                    Some(reader.error_position()),
                                    &format!(
                                        "XML declaration encoding `{encoding}` is not supported"
                                    ),
                                ));
                            } else if let Some(charset) = mime_charset {
                                let declared = xml_normalized_encoding(&encoding);
                                let charset = xml_normalized_encoding(charset);
                                if declared != charset
                                    && !(declared == "utf-8" && charset == "us-ascii")
                                    && !(declared == "us-ascii" && charset == "utf-8")
                                {
                                    diagnostics.push(xml_diagnostic(
                                        input,
                                        source,
                                        Some(reader.error_position()),
                                        "cem.xml.encoding_conflict",
                                        cem_ml::diagnostics::Severity::Warning,
                                        format!(
                                            "XML declaration encoding `{encoding}` conflicts with content-type charset `{charset}`"
                                        ),
                                    ));
                                }
                            }
                        }
                        Err(error) => diagnostics.push(xml_attribute_error_diagnostic(
                            input,
                            source,
                            &error,
                            Some(reader.error_position()),
                        )),
                    }
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => diagnostics.push(xml_diagnostic(
                input,
                source,
                Some(reader.error_position()),
                "cem.xml.dtd_rejected",
                cem_ml::diagnostics::Severity::Error,
                "XML DTD declarations are rejected until an explicit DTD policy enables them"
                    .to_owned(),
            )),
            Ok(quick_xml::events::Event::GeneralRef(reference)) => {
                if !xml_entity_reference_is_builtin(reference.as_ref()) {
                    diagnostics.push(xml_diagnostic(
                        input,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.external_entity_rejected",
                        cem_ml::diagnostics::Severity::Error,
                        format!(
                            "XML entity reference `&{};` is rejected",
                            String::from_utf8_lossy(reference.as_ref())
                        ),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Text(text)) => {
                if kind == XmlSourceKind::Document
                    && element_stack.is_empty()
                    && !xml_bytes_are_whitespace(text.as_ref())
                {
                    diagnostics.push(xml_diagnostic(
                        input,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        cem_ml::diagnostics::Severity::Error,
                        "XML document cannot contain character data outside the document element"
                            .to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::CData(_)) if kind == XmlSourceKind::Document => {
                if element_stack.is_empty() {
                    diagnostics.push(xml_diagnostic(
                        input,
                        source,
                        Some(reader.error_position()),
                        "cem.xml.parse_error",
                        cem_ml::diagnostics::Severity::Error,
                        "XML document cannot contain CDATA outside the document element".to_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                diagnostics.push(xml_reader_error_diagnostic(
                    input,
                    source,
                    Some(reader.error_position()),
                    &error,
                ));
                break;
            }
        }
    }

    if kind == XmlSourceKind::Document && root_count == 0 {
        diagnostics.push(xml_diagnostic(
            input,
            source,
            Some(0),
            "cem.xml.parse_error",
            cem_ml::diagnostics::Severity::Error,
            "XML document must contain a document element".to_owned(),
        ));
    }
    if let Some(unclosed) = element_stack.last() {
        diagnostics.push(xml_diagnostic(
            input,
            source,
            Some(reader.buffer_position()),
            "cem.xml.parse_error",
            cem_ml::diagnostics::Severity::Error,
            format!("XML start tag `<{unclosed}>` is missing a matching end tag"),
        ));
    }

    diagnostics
}

#[derive(Clone, Debug)]
struct XmlAttributeView {
    qualified_name: String,
}

fn validate_xml_start_event(
    input: &eng::EngineInput,
    source: &str,
    start: &quick_xml::events::BytesStart<'_>,
    namespace_stack: &[BTreeMap<String, String>],
) -> (
    BTreeMap<String, String>,
    Vec<cem_ml::diagnostics::Diagnostic>,
) {
    let mut diagnostics = Vec::new();
    let mut attributes = Vec::new();
    let mut next_namespaces = namespace_stack
        .last()
        .cloned()
        .unwrap_or_else(xml_initial_namespaces);

    for attribute in start.attributes().with_checks(false) {
        match attribute {
            Ok(attribute) => {
                let qualified_name = xml_qname_display(attribute.key.as_ref());
                let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                if qualified_name == "xmlns" {
                    next_namespaces.insert(String::new(), value.clone());
                } else if let Some(prefix) = qualified_name.strip_prefix("xmlns:") {
                    next_namespaces.insert(prefix.to_owned(), value.clone());
                }
                diagnostics.extend(xml_entity_reference_diagnostics(
                    input,
                    source,
                    value.as_bytes(),
                ));
                attributes.push(XmlAttributeView { qualified_name });
            }
            Err(error) => {
                diagnostics.push(xml_attribute_error_diagnostic(input, source, &error, None))
            }
        }
    }

    let element_name = xml_qname_display(start.name().as_ref());
    if let Some(prefix) = xml_qname_prefix(&element_name) {
        if !xml_prefix_is_bound(&next_namespaces, prefix) {
            diagnostics.push(xml_unbound_namespace_prefix_diagnostic(
                input,
                source,
                None,
                prefix,
                &element_name,
            ));
        }
    }

    let mut expanded_attributes = BTreeSet::new();
    for attribute in attributes {
        if xml_attribute_is_namespace_declaration(&attribute.qualified_name) {
            continue;
        }

        let (namespace_uri, local_name) =
            xml_attribute_expanded_name(&attribute.qualified_name, &next_namespaces);
        if let Some(prefix) = xml_qname_prefix(&attribute.qualified_name) {
            if !xml_prefix_is_bound(&next_namespaces, prefix) {
                diagnostics.push(xml_unbound_namespace_prefix_diagnostic(
                    input,
                    source,
                    None,
                    prefix,
                    &attribute.qualified_name,
                ));
            }
        }

        if !expanded_attributes.insert((namespace_uri.clone(), local_name.clone())) {
            diagnostics.push(xml_diagnostic(
                input,
                source,
                None,
                "cem.xml.duplicate_attribute",
                cem_ml::diagnostics::Severity::Error,
                format!(
                    "XML element `<{element_name}>` has a duplicate attribute `{}`",
                    attribute.qualified_name
                ),
            ));
        }
    }

    (next_namespaces, diagnostics)
}

fn xml_initial_namespaces() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "xml".to_owned(),
            "http://www.w3.org/XML/1998/namespace".to_owned(),
        ),
        (
            "xmlns".to_owned(),
            "http://www.w3.org/2000/xmlns/".to_owned(),
        ),
    ])
}

fn xml_attribute_expanded_name(
    qualified_name: &str,
    namespaces: &BTreeMap<String, String>,
) -> (String, String) {
    if let Some((prefix, local_name)) = qualified_name.split_once(':') {
        let namespace_uri = namespaces.get(prefix).cloned().unwrap_or_default();
        (namespace_uri, local_name.to_owned())
    } else {
        (String::new(), qualified_name.to_owned())
    }
}

fn xml_qname_prefix(qualified_name: &str) -> Option<&str> {
    qualified_name
        .split_once(':')
        .map(|(prefix, _)| prefix)
        .filter(|prefix| !prefix.is_empty() && *prefix != "xml")
}

fn xml_prefix_is_bound(namespaces: &BTreeMap<String, String>, prefix: &str) -> bool {
    namespaces
        .get(prefix)
        .is_some_and(|namespace| !namespace.trim().is_empty())
}

fn xml_attribute_is_namespace_declaration(qualified_name: &str) -> bool {
    qualified_name == "xmlns" || qualified_name.starts_with("xmlns:")
}

fn xml_qname_display(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}

fn xml_entity_reference_is_builtin(name: &[u8]) -> bool {
    name.starts_with(b"#") || matches!(name, b"amp" | b"lt" | b"gt" | b"apos" | b"quot")
}

fn xml_entity_reference_diagnostics(
    input: &eng::EngineInput,
    source: &str,
    value: &[u8],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut remaining = value;
    while let Some(start) = remaining.iter().position(|byte| *byte == b'&') {
        let after_amp = &remaining[start + 1..];
        let Some(end) = after_amp.iter().position(|byte| *byte == b';') else {
            break;
        };
        let reference = &after_amp[..end];
        if !xml_entity_reference_is_builtin(reference) {
            diagnostics.push(xml_diagnostic(
                input,
                source,
                None,
                "cem.xml.external_entity_rejected",
                cem_ml::diagnostics::Severity::Error,
                format!(
                    "XML entity reference `&{};` is rejected",
                    String::from_utf8_lossy(reference)
                ),
            ));
        }
        remaining = &after_amp[end + 1..];
    }
    diagnostics
}

fn xml_bytes_are_whitespace(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .map(|value| value.chars().all(char::is_whitespace))
        .unwrap_or(false)
}

fn xml_encoding_is_supported(encoding: &str) -> bool {
    matches!(
        xml_normalized_encoding(encoding).as_str(),
        "utf-8" | "us-ascii"
    )
}

fn xml_normalized_encoding(encoding: &str) -> String {
    encoding.trim().trim_matches('"').to_ascii_lowercase()
}

fn xml_event_position(
    reader: &quick_xml::Reader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
    empty: bool,
) -> Option<u64> {
    let markup_overhead = if empty { 3 } else { 2 };
    reader
        .buffer_position()
        .checked_sub(start.as_ref().len() as u64 + markup_overhead)
}

fn xml_reader_error_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: Option<u64>,
    error: &quick_xml::Error,
) -> cem_ml::diagnostics::Diagnostic {
    let code = match error {
        quick_xml::Error::Encoding(_) => "cem.xml.unsupported_encoding",
        quick_xml::Error::InvalidAttr(quick_xml::events::attributes::AttrError::Duplicated(
            _,
            _,
        )) => "cem.xml.duplicate_attribute",
        quick_xml::Error::Namespace(_) => "cem.xml.unbound_namespace_prefix",
        _ => "cem.xml.parse_error",
    };
    xml_diagnostic(
        input,
        source,
        byte_offset,
        code,
        cem_ml::diagnostics::Severity::Error,
        format!("XML parse error: {error}"),
    )
}

fn xml_attribute_error_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    error: &quick_xml::events::attributes::AttrError,
    base_offset: Option<u64>,
) -> cem_ml::diagnostics::Diagnostic {
    let code = match error {
        quick_xml::events::attributes::AttrError::Duplicated(_, _) => "cem.xml.duplicate_attribute",
        _ => "cem.xml.parse_error",
    };
    xml_diagnostic(
        input,
        source,
        base_offset,
        code,
        cem_ml::diagnostics::Severity::Error,
        format!("XML attribute parse error: {error}"),
    )
}

fn xml_unbound_namespace_prefix_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: Option<u64>,
    prefix: &str,
    qualified_name: &str,
) -> cem_ml::diagnostics::Diagnostic {
    xml_diagnostic(
        input,
        source,
        byte_offset,
        "cem.xml.unbound_namespace_prefix",
        cem_ml::diagnostics::Severity::Error,
        format!("XML namespace prefix `{prefix}` is not bound for `{qualified_name}`"),
    )
}

fn xml_unsupported_utf8_diagnostic(
    input: &eng::EngineInput,
    error: &std::str::Utf8Error,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        byte_offset: u64::try_from(error.valid_up_to()).ok(),
        code: "cem.xml.unsupported_encoding".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("XML source must be valid UTF-8: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn xml_unsupported_encoding_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: Option<u64>,
    message: &str,
) -> cem_ml::diagnostics::Diagnostic {
    xml_diagnostic(
        input,
        source,
        byte_offset,
        "cem.xml.unsupported_encoding",
        cem_ml::diagnostics::Severity::Error,
        message.to_owned(),
    )
}

fn xml_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: Option<u64>,
    code: &'static str,
    severity: cem_ml::diagnostics::Severity,
    message: String,
) -> cem_ml::diagnostics::Diagnostic {
    let (line, column) = byte_offset
        .and_then(|offset| usize::try_from(offset).ok())
        .map(|offset| markdown_line_col(source, offset))
        .map(|(line, column)| (Some(line), Some(column)))
        .unwrap_or((None, None));
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line,
        column,
        byte_offset,
        code: code.to_owned(),
        severity,
        message,
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn collect_cem_dom_projection_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        match cem_dom_projection_source_kind(input) {
            CemDomProjectionSourceKind::Binary => {
                diagnostics.extend(validate_cem_dom_projection_binary(input));
            }
            CemDomProjectionSourceKind::Json => {
                diagnostics.extend(validate_cem_dom_projection_json(input));
            }
        }
    }
    diagnostics
}

fn collect_cem_ast_projection_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        match cem_ast_projection_source_kind(input) {
            CemAstProjectionSourceKind::Binary => {
                diagnostics.extend(validate_cem_ast_projection_binary(input));
            }
            CemAstProjectionSourceKind::Json => {
                diagnostics.extend(validate_cem_ast_projection_json(input));
            }
        }
    }
    diagnostics
}

fn collect_cem_events_projection_source_diagnostics(
    inputs: &[eng::EngineInput],
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        match cem_events_projection_source_kind(input) {
            CemEventsProjectionSourceKind::Binary => {
                diagnostics.extend(validate_cem_events_projection_binary(input));
            }
            CemEventsProjectionSourceKind::Json => {
                diagnostics.extend(validate_cem_events_projection_json(input));
            }
        }
    }
    diagnostics
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CemDomProjectionSourceKind {
    Binary,
    Json,
}

fn cem_dom_projection_source_kind(input: &eng::EngineInput) -> CemDomProjectionSourceKind {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type)
            if content_type == cem_ml::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE =>
        {
            CemDomProjectionSourceKind::Json
        }
        Some(content_type)
            if content_type == cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE =>
        {
            CemDomProjectionSourceKind::Binary
        }
        _ if input.bytes.starts_with(b"CEMPROJ\0") => CemDomProjectionSourceKind::Binary,
        _ => CemDomProjectionSourceKind::Json,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CemAstProjectionSourceKind {
    Binary,
    Json,
}

fn cem_ast_projection_source_kind(input: &eng::EngineInput) -> CemAstProjectionSourceKind {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type)
            if content_type == cem_ml::schema::registry::CEM_AST_JSON_PROJECTION_CONTENT_TYPE =>
        {
            CemAstProjectionSourceKind::Json
        }
        Some(content_type)
            if content_type == cem_ml::schema::registry::CEM_AST_PROJECTION_CONTENT_TYPE =>
        {
            CemAstProjectionSourceKind::Binary
        }
        _ if input.bytes.starts_with(b"CEMPROJ\0") => CemAstProjectionSourceKind::Binary,
        _ => CemAstProjectionSourceKind::Json,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CemEventsProjectionSourceKind {
    Binary,
    Json,
}

fn cem_events_projection_source_kind(input: &eng::EngineInput) -> CemEventsProjectionSourceKind {
    let identity = input
        .identity
        .clone()
        .unwrap_or_else(|| input.root_scope.format_identity());
    let content_type = identity
        .content_type
        .as_deref()
        .map(cli_content_type_essence);

    match content_type.as_deref() {
        Some(content_type)
            if content_type
                == cem_ml::schema::registry::CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE =>
        {
            CemEventsProjectionSourceKind::Json
        }
        Some(content_type)
            if content_type == cem_ml::schema::registry::CEM_EVENTS_PROJECTION_CONTENT_TYPE =>
        {
            CemEventsProjectionSourceKind::Binary
        }
        _ if input.bytes.starts_with(b"CEMPROJ\0") => CemEventsProjectionSourceKind::Binary,
        _ => CemEventsProjectionSourceKind::Json,
    }
}

fn validate_cem_dom_projection_binary(
    input: &eng::EngineInput,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    match validate_cem_dom_projection_binary_bytes(&input.bytes) {
        Ok(()) => Vec::new(),
        Err((code, message)) => vec![cem_dom_projection_diagnostic(input, code, message)],
    }
}

fn validate_cem_ast_projection_binary(
    input: &eng::EngineInput,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    match validate_cem_ast_projection_binary_bytes(&input.bytes) {
        Ok(()) => Vec::new(),
        Err((code, message)) => vec![cem_projection_diagnostic(input, code, message)],
    }
}

fn validate_cem_events_projection_binary(
    input: &eng::EngineInput,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    match validate_cem_events_projection_binary_bytes(&input.bytes) {
        Ok(()) => Vec::new(),
        Err((code, message)) => vec![cem_projection_diagnostic(input, code, message)],
    }
}

fn validate_cem_dom_projection_binary_bytes(bytes: &[u8]) -> Result<(), (&'static str, String)> {
    if !bytes.starts_with(b"CEMPROJ\0") {
        return Err((
            "cem.projection.dom.binary_magic",
            "CEM DOM binary projection must start with CEMPROJ\\0 magic".to_owned(),
        ));
    }

    let mut reader = ProjectionBinaryReader::new(
        &bytes[b"CEMPROJ\0".len()..],
        "cem.projection.dom",
        "CEM DOM",
    );
    let version = reader.read_u16("version")?;
    if version != 1 {
        return Err((
            "cem.projection.dom.binary_version",
            format!("unsupported CEM projection binary version `{version}`; expected `1`"),
        ));
    }

    let projection_kind = reader.read_u8("projection kind")?;
    if projection_kind != 1 {
        return Err((
            "cem.projection.dom.projection_mismatch",
            format!("binary projection kind `{projection_kind}` is not CEM DOM kind `1`"),
        ));
    }

    let schema = reader.read_str("schema")?;
    if schema != cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI {
        return Err((
            "cem.projection.dom.projection_mismatch",
            format!(
                "binary projection schema `{schema}` is not `{}`",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI
            ),
        ));
    }

    let content_type = reader.read_str("content type")?;
    if content_type != cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE {
        return Err((
            "cem.projection.dom.projection_mismatch",
            format!(
                "binary projection content type `{content_type}` is not `{}`",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE
            ),
        ));
    }

    let _node_count = reader.read_u32("node count")?;
    Ok(())
}

fn validate_cem_ast_projection_binary_bytes(bytes: &[u8]) -> Result<(), (&'static str, String)> {
    if !bytes.starts_with(b"CEMPROJ\0") {
        return Err((
            "cem.projection.ast.binary_magic",
            "CEM AST binary projection must start with CEMPROJ\\0 magic".to_owned(),
        ));
    }

    let mut reader = ProjectionBinaryReader::new(
        &bytes[b"CEMPROJ\0".len()..],
        "cem.projection.ast",
        "CEM AST",
    );
    let version = reader.read_u16("version")?;
    if version != 1 {
        return Err((
            "cem.projection.ast.binary_version",
            format!("unsupported CEM projection binary version `{version}`; expected `1`"),
        ));
    }

    let projection_kind = reader.read_u8("projection kind")?;
    if projection_kind != 2 {
        return Err((
            "cem.projection.ast.projection_mismatch",
            format!("binary projection kind `{projection_kind}` is not CEM AST kind `2`"),
        ));
    }

    let schema = reader.read_str("schema")?;
    if schema != cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI {
        return Err((
            "cem.projection.ast.projection_mismatch",
            format!(
                "binary projection schema `{schema}` is not `{}`",
                cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI
            ),
        ));
    }

    let content_type = reader.read_str("content type")?;
    if content_type != cem_ml::schema::registry::CEM_AST_PROJECTION_CONTENT_TYPE {
        return Err((
            "cem.projection.ast.projection_mismatch",
            format!(
                "binary projection content type `{content_type}` is not `{}`",
                cem_ml::schema::registry::CEM_AST_PROJECTION_CONTENT_TYPE
            ),
        ));
    }

    let _node_count = reader.read_u32("node count")?;
    Ok(())
}

fn validate_cem_events_projection_binary_bytes(bytes: &[u8]) -> Result<(), (&'static str, String)> {
    if !bytes.starts_with(b"CEMPROJ\0") {
        return Err((
            "cem.projection.events.binary_magic",
            "CEM events binary projection must start with CEMPROJ\\0 magic".to_owned(),
        ));
    }

    let mut reader = ProjectionBinaryReader::new(
        &bytes[b"CEMPROJ\0".len()..],
        "cem.projection.events",
        "CEM events",
    );
    let version = reader.read_u16("version")?;
    if version != 1 {
        return Err((
            "cem.projection.events.binary_version",
            format!("unsupported CEM projection binary version `{version}`; expected `1`"),
        ));
    }

    let projection_kind = reader.read_u8("projection kind")?;
    if projection_kind != 3 {
        return Err((
            "cem.projection.events.projection_mismatch",
            format!("binary projection kind `{projection_kind}` is not CEM events kind `3`"),
        ));
    }

    let schema = reader.read_str("schema")?;
    if schema != cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI {
        return Err((
            "cem.projection.events.projection_mismatch",
            format!(
                "binary projection schema `{schema}` is not `{}`",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI
            ),
        ));
    }

    let content_type = reader.read_str("content type")?;
    if content_type != cem_ml::schema::registry::CEM_EVENTS_PROJECTION_CONTENT_TYPE {
        return Err((
            "cem.projection.events.projection_mismatch",
            format!(
                "binary projection content type `{content_type}` is not `{}`",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_CONTENT_TYPE
            ),
        ));
    }

    let _event_count = reader.read_u32("event count")?;
    Ok(())
}

struct ProjectionBinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    diagnostic_prefix: &'static str,
    label: &'static str,
}

impl<'a> ProjectionBinaryReader<'a> {
    fn new(bytes: &'a [u8], diagnostic_prefix: &'static str, label: &'static str) -> Self {
        Self {
            bytes,
            offset: 0,
            diagnostic_prefix,
            label,
        }
    }

    fn read_u8(&mut self, field: &'static str) -> Result<u8, (&'static str, String)> {
        let bytes = self.read_exact(field, 1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self, field: &'static str) -> Result<u16, (&'static str, String)> {
        let bytes = self.read_exact(field, 2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self, field: &'static str) -> Result<u32, (&'static str, String)> {
        let bytes = self.read_exact(field, 4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_str(&mut self, field: &'static str) -> Result<String, (&'static str, String)> {
        let len = self.read_u32(field)? as usize;
        let bytes = self.read_exact(field, len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| {
                (
                    self.binary_truncated_code(),
                    format!(
                        "{} binary projection {field} is not UTF-8: {error}",
                        self.label
                    ),
                )
            })
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        len: usize,
    ) -> Result<&'a [u8], (&'static str, String)> {
        let end = self.offset.checked_add(len).ok_or_else(|| {
            (
                self.binary_truncated_code(),
                format!(
                    "{} binary projection {field} length overflows input",
                    self.label
                ),
            )
        })?;
        if end > self.bytes.len() {
            return Err((
                self.binary_truncated_code(),
                format!(
                    "{} binary projection is truncated while reading {field}",
                    self.label
                ),
            ));
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn binary_truncated_code(&self) -> &'static str {
        match self.diagnostic_prefix {
            "cem.projection.ast" => "cem.projection.ast.binary_truncated",
            "cem.projection.events" => "cem.projection.events.binary_truncated",
            _ => "cem.projection.dom.binary_truncated",
        }
    }
}

fn validate_cem_dom_projection_json(
    input: &eng::EngineInput,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let value = match serde_json::from_slice::<serde_json::Value>(&input.bytes) {
        Ok(value) => value,
        Err(error) => {
            return vec![cem_dom_projection_diagnostic(
                input,
                "cem.projection.dom.json_parse_error",
                format!("CEM DOM JSON projection parse error: {error}"),
            )];
        }
    };

    match validate_cem_dom_projection_json_value(&value) {
        Ok(()) => Vec::new(),
        Err(message) => vec![cem_dom_projection_diagnostic(
            input,
            "cem.projection.dom.json_shape",
            message,
        )],
    }
}

fn validate_cem_ast_projection_json(
    input: &eng::EngineInput,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let value = match serde_json::from_slice::<serde_json::Value>(&input.bytes) {
        Ok(value) => value,
        Err(error) => {
            return vec![cem_projection_diagnostic(
                input,
                "cem.projection.ast.json_parse_error",
                format!("CEM AST JSON projection parse error: {error}"),
            )];
        }
    };

    match validate_cem_ast_projection_json_value(&value) {
        Ok(()) => Vec::new(),
        Err(message) => vec![cem_projection_diagnostic(
            input,
            "cem.projection.ast.json_shape",
            message,
        )],
    }
}

fn validate_cem_events_projection_json(
    input: &eng::EngineInput,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let value = match serde_json::from_slice::<serde_json::Value>(&input.bytes) {
        Ok(value) => value,
        Err(error) => {
            return vec![cem_projection_diagnostic(
                input,
                "cem.projection.events.json_parse_error",
                format!("CEM events JSON projection parse error: {error}"),
            )];
        }
    };

    match validate_cem_events_projection_json_value(&value) {
        Ok(()) => Vec::new(),
        Err(message) => vec![cem_projection_diagnostic(
            input,
            "cem.projection.events.json_shape",
            message,
        )],
    }
}

fn validate_cem_dom_projection_json_value(value: &serde_json::Value) -> Result<(), String> {
    validate_cem_tree_projection_json_value(
        value,
        "CEM DOM",
        "dom",
        cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI,
        cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE,
    )
}

fn validate_cem_ast_projection_json_value(value: &serde_json::Value) -> Result<(), String> {
    validate_cem_tree_projection_json_value(
        value,
        "CEM AST",
        "ast",
        cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI,
        cem_ml::schema::registry::CEM_AST_PROJECTION_CONTENT_TYPE,
    )
}

fn validate_cem_events_projection_json_value(value: &serde_json::Value) -> Result<(), String> {
    if let Some(object) = value.as_object() {
        if object.get("kind").and_then(serde_json::Value::as_str) == Some("cem-binary-projection") {
            return validate_cem_binary_projection_json(
                object,
                "events",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI,
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_CONTENT_TYPE,
            );
        }
    }

    let events = value
        .as_array()
        .ok_or_else(|| "CEM events JSON projection root must be an event array".to_owned())?;
    for (index, event) in events.iter().enumerate() {
        validate_cem_event_json(event, &format!("$[{index}]"))?;
    }
    Ok(())
}

fn validate_cem_event_json(value: &serde_json::Value, path: &str) -> Result<(), String> {
    let object = json_object(value, path)?;
    let Some(kind) = object.get("kind").and_then(serde_json::Value::as_str) else {
        return Err(format!("{path}.kind must be a string"));
    };

    match kind {
        "open" | "close" | "name" => {
            expect_json_string_field(object, "name", path, None)?;
            validate_required_byte_range(object, path)
        }
        "value" => {
            let value = object
                .get("value")
                .ok_or_else(|| format!("{path}.value is required"))?;
            if value.is_array() || value.is_object() {
                return Err(format!(
                    "{path}.value must be a string, number, boolean, or null"
                ));
            }
            validate_required_byte_range(object, path)
        }
        "trivia" => {
            let trivia = expect_json_string_field(object, "trivia", path, None)?;
            if !matches!(trivia, "whitespace" | "comment") {
                return Err(format!(
                    "{path}.trivia `{trivia}` is not a supported trivia kind"
                ));
            }
            expect_json_string_field(object, "data", path, None)?;
            validate_required_byte_range(object, path)
        }
        "processing-instruction" => {
            expect_json_string_field(object, "target", path, None)?;
            expect_json_string_field(object, "data", path, None)?;
            validate_required_byte_range(object, path)
        }
        "separator" => validate_required_byte_range(object, path),
        "mode-switch" => {
            expect_json_string_field(object, "contentType", path, None)?;
            Ok(())
        }
        "error" => {
            expect_json_string_field(object, "code", path, None)?;
            validate_required_byte_range(object, path)
        }
        _ => Err(format!(
            "{path}.kind `{kind}` is not a supported CEM events kind"
        )),
    }
}

fn validate_cem_tree_projection_json_value(
    value: &serde_json::Value,
    label: &str,
    projection: &str,
    schema_uri: &str,
    content_type: &str,
) -> Result<(), String> {
    let object = json_object(value, "$")?;
    match object.get("kind").and_then(serde_json::Value::as_str) {
        Some("document") => validate_cem_tree_json_node(value, "$", label),
        Some("cem-binary-projection") => {
            validate_cem_binary_projection_json(object, projection, schema_uri, content_type)
        }
        Some(kind) => Err(format!(
            "{label} JSON projection root kind `{kind}` is not `document` or `cem-binary-projection`"
        )),
        None => Err(format!("{label} JSON projection root requires string `kind`")),
    }
}

fn validate_cem_binary_projection_json(
    object: &serde_json::Map<String, serde_json::Value>,
    projection: &str,
    schema_uri: &str,
    content_type: &str,
) -> Result<(), String> {
    expect_json_string_field(object, "projection", "$", Some(projection))?;
    expect_json_string_field(object, "schema", "$", Some(schema_uri))?;
    expect_json_string_field(object, "contentType", "$", Some(content_type))?;
    expect_json_string_field(object, "formatVersion", "$", Some("cem-projection-bin/1"))?;
    expect_json_string_field(object, "hashScheme", "$", None)?;
    expect_json_string_field(object, "hash", "$", None)?;
    expect_json_u64_field(object, "byteLength", "$")?;
    if let Some(native_bytes) = object.get("nativeBytes") {
        if !native_bytes.is_boolean() {
            return Err("$.nativeBytes must be a boolean".to_owned());
        }
    }
    if let Some(chunks) = object.get("chunks") {
        validate_cem_dom_projection_json_chunks(chunks)?;
    }
    Ok(())
}

fn validate_cem_dom_projection_json_chunks(chunks: &serde_json::Value) -> Result<(), String> {
    let chunks = chunks
        .as_array()
        .ok_or_else(|| "$.chunks must be an array".to_owned())?;
    let mut expected_offset = 0_u64;
    for (index, chunk) in chunks.iter().enumerate() {
        let path = format!("$.chunks[{index}]");
        let object = json_object(chunk, &path)?;
        expect_json_string_field(object, "id", &path, None)?;
        if object.get("sealed").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err(format!("{path}.sealed must be true"));
        }
        let offset = expect_json_u64_field(object, "byteOffset", &path)?;
        if offset != expected_offset {
            return Err(format!(
                "{path}.byteOffset must be contiguous at {expected_offset}"
            ));
        }
        let byte_length = expect_json_u64_field(object, "byteLength", &path)?;
        expect_json_string_field(object, "hash", &path, None)?;
        expect_json_string_field(object, "dataEncoding", &path, Some("hex"))?;
        let data = expect_json_string_field(object, "data", &path, None)?;
        if data.len() % 2 != 0 || !data.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("{path}.data must be even-length hex"));
        }
        if (data.len() / 2) as u64 != byte_length {
            return Err(format!(
                "{path}.byteLength does not match decoded hex data length"
            ));
        }
        expected_offset += byte_length;
    }
    Ok(())
}

fn validate_cem_tree_json_node(
    value: &serde_json::Value,
    path: &str,
    label: &str,
) -> Result<(), String> {
    let object = json_object(value, path)?;
    let Some(kind) = object.get("kind").and_then(serde_json::Value::as_str) else {
        return Err(format!("{path}.kind must be a string"));
    };

    match kind {
        "document" => validate_cem_tree_json_children(object, path, label),
        "element" => {
            expect_json_string_field(object, "name", path, None)?;
            if object.contains_key("namespace") {
                expect_json_string_field(object, "namespace", path, None)?;
            }
            validate_cem_dom_json_attributes(object, path)?;
            validate_cem_tree_json_children(object, path, label)?;
            validate_optional_byte_range(object, path)
        }
        "text" | "whitespace" | "comment" | "cdata" | "raw-text" => {
            expect_json_string_field(object, "data", path, None)?;
            validate_optional_byte_range(object, path)
        }
        "processing-instruction" => {
            expect_json_string_field(object, "target", path, None)?;
            expect_json_string_field(object, "data", path, None)?;
            validate_optional_byte_range(object, path)
        }
        "error" => {
            expect_json_string_field(object, "code", path, None)?;
            validate_optional_byte_range(object, path)
        }
        _ => Err(format!(
            "{path}.kind `{kind}` is not a supported {label} node kind"
        )),
    }
}

fn validate_cem_tree_json_children(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    label: &str,
) -> Result<(), String> {
    let children_path = format!("{path}.children");
    let children = object
        .get("children")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("{children_path} must be an array"))?;
    for (index, child) in children.iter().enumerate() {
        validate_cem_tree_json_node(child, &format!("{children_path}[{index}]"), label)?;
    }
    Ok(())
}

fn validate_cem_dom_json_attributes(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), String> {
    let attrs_path = format!("{path}.attributes");
    let attrs = object
        .get("attributes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("{attrs_path} must be an array"))?;
    for (index, attr) in attrs.iter().enumerate() {
        let attr_path = format!("{attrs_path}[{index}]");
        let attr = json_object(attr, &attr_path)?;
        expect_json_string_field(attr, "name", &attr_path, None)?;
        if attr.contains_key("namespace") {
            expect_json_string_field(attr, "namespace", &attr_path, None)?;
        }
        if let Some(value) = attr.get("value") {
            if !value.is_null() && !value.is_string() {
                return Err(format!("{attr_path}.value must be a string or null"));
            }
        }
        validate_optional_byte_range(attr, &attr_path)?;
    }
    Ok(())
}

fn validate_optional_byte_range(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), String> {
    let Some(byte_range) = object.get("byteRange") else {
        return Ok(());
    };
    if byte_range.is_null() {
        return Ok(());
    }
    let range_path = format!("{path}.byteRange");
    let range = json_object(byte_range, &range_path)?;
    expect_json_u64_field(range, "start", &range_path)?;
    expect_json_u64_field(range, "len", &range_path)?;
    Ok(())
}

fn validate_required_byte_range(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), String> {
    if !object.contains_key("byteRange") {
        return Err(format!("{path}.byteRange is required"));
    }
    validate_optional_byte_range(object, path)
}

fn json_object<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))
}

fn expect_json_string_field<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    path: &str,
    expected: Option<&str>,
) -> Result<&'a str, String> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{path}.{field} must be a string"))?;
    if let Some(expected) = expected {
        if value != expected {
            return Err(format!(
                "{path}.{field} must be `{expected}`, got `{value}`"
            ));
        }
    }
    Ok(value)
}

fn expect_json_u64_field(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    path: &str,
) -> Result<u64, String> {
    object
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("{path}.{field} must be a non-negative integer"))
}

fn cem_dom_projection_diagnostic(
    input: &eng::EngineInput,
    code: &'static str,
    message: String,
) -> cem_ml::diagnostics::Diagnostic {
    cem_projection_diagnostic(input, code, message)
}

fn cem_projection_diagnostic(
    input: &eng::EngineInput,
    code: &'static str,
    message: String,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        code: code.to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message,
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn json_parse_error_diagnostic(
    input: &eng::EngineInput,
    error: &serde_json::Error,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: u32::try_from(error.line()).ok(),
        column: u32::try_from(error.column()).ok(),
        code: "cem.json.parse_error".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("JSON parse error: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn json_schema_parse_error_diagnostic(
    input: &eng::EngineInput,
    error: &serde_json::Error,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: u32::try_from(error.line()).ok(),
        column: u32::try_from(error.column()).ok(),
        code: "cem.json_schema.parse_error".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("JSON Schema parse error: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn yaml_parse_error_diagnostic(
    input: &eng::EngineInput,
    error: &yaml_rust2::scanner::ScanError,
) -> cem_ml::diagnostics::Diagnostic {
    let marker = error.marker();
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: u32::try_from(marker.line()).ok(),
        column: u32::try_from(marker.col()).ok(),
        code: "cem.yaml.parse_error".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("YAML parse error: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn yaml_unsupported_encoding_diagnostic(
    input: &eng::EngineInput,
    error: &std::str::Utf8Error,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        code: "cem.yaml.unsupported_encoding".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("YAML source must be valid UTF-8: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

struct YamlValidationReceiver<'a> {
    input: &'a eng::EngineInput,
    diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
}

impl yaml_rust2::parser::MarkedEventReceiver for YamlValidationReceiver<'_> {
    fn on_event(&mut self, ev: yaml_rust2::parser::Event, marker: yaml_rust2::scanner::Marker) {
        match ev {
            yaml_rust2::parser::Event::Scalar(_, _, _, Some(tag))
            | yaml_rust2::parser::Event::SequenceStart(_, Some(tag))
            | yaml_rust2::parser::Event::MappingStart(_, Some(tag)) => {
                if !is_safe_yaml_tag(&tag) {
                    self.diagnostics.push(yaml_unsafe_tag_diagnostic(
                        self.input,
                        &marker,
                        &yaml_tag_display(&tag),
                    ));
                }
            }
            _ => {}
        }
    }
}

fn is_safe_yaml_tag(tag: &yaml_rust2::parser::Tag) -> bool {
    let handle = tag.handle.trim();
    let suffix = tag.suffix.trim();
    if handle.is_empty() && suffix.is_empty() {
        return true;
    }

    match handle {
        "!" => suffix.is_empty(),
        "!!" | "tag:yaml.org,2002:" => is_safe_yaml_core_tag_name(suffix),
        _ => false,
    }
}

fn is_safe_yaml_core_tag_name(name: &str) -> bool {
    matches!(
        name,
        "binary"
            | "bool"
            | "float"
            | "int"
            | "map"
            | "merge"
            | "null"
            | "omap"
            | "pairs"
            | "seq"
            | "set"
            | "str"
            | "timestamp"
            | "value"
            | "yaml"
    )
}

fn yaml_unsafe_tag_diagnostic(
    input: &eng::EngineInput,
    marker: &yaml_rust2::scanner::Marker,
    tag: &str,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: u32::try_from(marker.line()).ok(),
        column: u32::try_from(marker.col()).ok(),
        code: "cem.yaml.unsafe_tag".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("YAML node uses unsupported explicit tag `{tag}`"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn yaml_tag_display(tag: &yaml_rust2::parser::Tag) -> String {
    format!("{}{}", tag.handle, tag.suffix)
}

fn csv_parse_error_diagnostic(
    input: &eng::EngineInput,
    error: &csv::Error,
) -> cem_ml::diagnostics::Diagnostic {
    let code = csv_parse_error_code(error);
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: csv_position_line(error.position()),
        column: None,
        byte_offset: csv_position_byte_offset(error.position()),
        code: code.to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("CSV parse error: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CsvSourcePosition {
    line: u32,
    column: u32,
    byte_offset: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CsvQuoteState {
    StartField,
    InUnquoted,
    InQuoted { open: CsvSourcePosition },
    AfterQuote,
}

fn collect_csv_quote_policy_diagnostics(
    input: &eng::EngineInput,
    source: &str,
) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let bytes = source.as_bytes();
    let mut diagnostics = Vec::new();
    let mut state = CsvQuoteState::StartField;
    let mut byte = 0usize;
    let mut line = 1u32;
    let mut column = 1u32;

    while byte < bytes.len() {
        let current = CsvSourcePosition {
            line,
            column,
            byte_offset: byte as u64,
        };
        match state {
            CsvQuoteState::StartField => match bytes[byte] {
                b'"' => {
                    state = CsvQuoteState::InQuoted { open: current };
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b',' => {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b'\r' | b'\n' => {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                    state = CsvQuoteState::StartField;
                }
                _ => {
                    state = CsvQuoteState::InUnquoted;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            },
            CsvQuoteState::InUnquoted => match bytes[byte] {
                b'"' => {
                    diagnostics.push(csv_quote_policy_diagnostic(
                        input,
                        "cem.csv.invalid_quote_escape",
                        current,
                        "CSV quote appears inside an unquoted field".to_owned(),
                    ));
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b',' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b'\r' | b'\n' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                _ => {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            },
            CsvQuoteState::InQuoted { .. } => {
                if bytes[byte] == b'"' {
                    if bytes.get(byte + 1) == Some(&b'"') {
                        byte += 2;
                        column = column.saturating_add(2);
                    } else {
                        state = CsvQuoteState::AfterQuote;
                        advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                    }
                } else {
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            }
            CsvQuoteState::AfterQuote => match bytes[byte] {
                b',' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                b'\r' | b'\n' => {
                    state = CsvQuoteState::StartField;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
                _ => {
                    diagnostics.push(csv_quote_policy_diagnostic(
                        input,
                        "cem.csv.invalid_quote_escape",
                        current,
                        "CSV quoted field has non-delimiter content after the closing quote"
                            .to_owned(),
                    ));
                    state = CsvQuoteState::InUnquoted;
                    advance_csv_cursor(bytes, &mut byte, &mut line, &mut column);
                }
            },
        }
    }

    if let CsvQuoteState::InQuoted { open } = state {
        diagnostics.push(csv_quote_policy_diagnostic(
            input,
            "cem.csv.unclosed_quote",
            open,
            "CSV quoted field is missing a closing quote".to_owned(),
        ));
    }

    diagnostics
}

fn advance_csv_cursor(bytes: &[u8], byte: &mut usize, line: &mut u32, column: &mut u32) {
    match bytes.get(*byte).copied() {
        Some(b'\r') => {
            if bytes.get(*byte + 1) == Some(&b'\n') {
                *byte += 2;
            } else {
                *byte += 1;
            }
            *line = line.saturating_add(1);
            *column = 1;
        }
        Some(b'\n') => {
            *byte += 1;
            *line = line.saturating_add(1);
            *column = 1;
        }
        Some(_) => {
            *byte += 1;
            *column = column.saturating_add(1);
        }
        None => {}
    }
}

fn csv_quote_policy_diagnostic(
    input: &eng::EngineInput,
    code: &'static str,
    position: CsvSourcePosition,
    message: String,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: Some(position.line),
        column: Some(position.column),
        byte_offset: Some(position.byte_offset),
        code: code.to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message,
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn csv_parse_error_code(error: &csv::Error) -> &'static str {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("utf-8") || message.contains("utf8") {
        "cem.csv.unsupported_encoding"
    } else if (message.contains("eof") || message.contains("end of file"))
        && message.contains("quote")
    {
        "cem.csv.unclosed_quote"
    } else if message.contains("quote") {
        "cem.csv.invalid_quote_escape"
    } else {
        "cem.csv.parse_error"
    }
}

fn csv_unsupported_encoding_diagnostic(
    input: &eng::EngineInput,
    error: &std::str::Utf8Error,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        byte_offset: u64::try_from(error.valid_up_to()).ok(),
        code: "cem.csv.unsupported_encoding".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("CSV source must be valid UTF-8: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn csv_inconsistent_field_count_diagnostic(
    input: &eng::EngineInput,
    position: Option<&csv::Position>,
    row_index: usize,
    expected: usize,
    actual: usize,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: csv_position_line(position),
        column: None,
        byte_offset: csv_position_byte_offset(position),
        code: "cem.csv.inconsistent_field_count".to_owned(),
        severity: cem_ml::diagnostics::Severity::Warning,
        message: format!(
            "CSV row {row_index} has {actual} fields; expected {expected} from the first row"
        ),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn csv_position_line(position: Option<&csv::Position>) -> Option<u32> {
    position.and_then(|position| u32::try_from(position.line()).ok())
}

fn csv_position_byte_offset(position: Option<&csv::Position>) -> Option<u64> {
    position.map(csv::Position::byte)
}

fn markdown_content_type_missing_charset(content_type: &str) -> bool {
    cli_content_type_essence(content_type) == cem_ml::schema::registry::MARKDOWN_CONTENT_TYPE
        && cli_content_type_parameter(content_type, "charset").is_none()
}

fn markdown_variant_is_known(variant: &str) -> bool {
    matches!(
        variant.trim().to_ascii_lowercase().as_str(),
        "commonmark" | "gfm" | "github-flavored-markdown"
    )
}

fn markdown_parser_options(variant: Option<&str>) -> pulldown_cmark::Options {
    let mut options = pulldown_cmark::Options::empty();
    if variant.map(str::trim).is_some_and(|variant| {
        variant.eq_ignore_ascii_case("gfm")
            || variant.eq_ignore_ascii_case("github-flavored-markdown")
    }) {
        options.insert(pulldown_cmark::Options::ENABLE_GFM);
    }
    options
}

fn markdown_charset_missing_diagnostic(
    input: &eng::EngineInput,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        code: "cem.markdown.charset_missing".to_owned(),
        severity: cem_ml::diagnostics::Severity::Warning,
        message: "text/markdown content type should include an explicit charset parameter"
            .to_owned(),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn markdown_unknown_variant_diagnostic(
    input: &eng::EngineInput,
    variant: &str,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        code: "cem.markdown.unknown_variant".to_owned(),
        severity: cem_ml::diagnostics::Severity::Warning,
        message: format!("unknown Markdown variant `{variant}`"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn markdown_unsupported_encoding_diagnostic(
    input: &eng::EngineInput,
    error: &std::str::Utf8Error,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        byte_offset: u64::try_from(error.valid_up_to()).ok(),
        code: "cem.markdown.unsupported_encoding".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: format!("Markdown source must be valid UTF-8: {error}"),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn markdown_embedded_html_rejected_diagnostic(
    input: &eng::EngineInput,
    source: &str,
    byte_offset: usize,
) -> cem_ml::diagnostics::Diagnostic {
    let (line, column) = markdown_line_col(source, byte_offset);
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        line: Some(line),
        column: Some(column),
        byte_offset: Some(byte_offset as u64),
        code: "cem.markdown.embedded_html_rejected".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: "Markdown embedded HTML is rejected unless an explicit policy permits it"
            .to_owned(),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn markdown_line_col(source: &str, byte_offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut column = 1u32;
    let limit = byte_offset.min(source.len());
    for byte in source[..limit].bytes() {
        if byte == b'\n' {
            line = line.saturating_add(1);
            column = 1;
        } else {
            column = column.saturating_add(1);
        }
    }
    (line, column)
}

fn json_schema_dialect_diagnostic(
    input: &eng::EngineInput,
    value: &serde_json::Value,
) -> Option<cem_ml::diagnostics::Diagnostic> {
    let serde_json::Value::Object(object) = value else {
        return Some(json_schema_unsupported_dialect_diagnostic(
            input,
            "JSON Schema document must be an object with a `$schema` dialect declaration",
        ));
    };
    match object.get("$schema").and_then(serde_json::Value::as_str) {
        Some("https://json-schema.org/draft/2020-12/schema")
        | Some("https://json-schema.org/draft/2020-12/schema#") => None,
        Some(dialect) => Some(json_schema_unsupported_dialect_diagnostic(
            input,
            &format!("unsupported JSON Schema dialect `{dialect}`; expected Draft 2020-12"),
        )),
        None => Some(json_schema_unsupported_dialect_diagnostic(
            input,
            "JSON Schema object is missing required `$schema` dialect declaration",
        )),
    }
}

fn json_schema_unsupported_dialect_diagnostic(
    input: &eng::EngineInput,
    message: &str,
) -> cem_ml::diagnostics::Diagnostic {
    cem_ml::diagnostics::Diagnostic {
        uri: Some(input.uri.clone()),
        code: "cem.json_schema.unsupported_dialect".to_owned(),
        severity: cem_ml::diagnostics::Severity::Error,
        message: message.to_owned(),
        ..cem_ml::diagnostics::Diagnostic::default()
    }
}

fn collect_fixture_embedding_diagnostics(
    context: &eng::EngineContext,
    inputs: &[eng::EngineInput],
) -> Result<Vec<cem_ml::diagnostics::Diagnostic>, EngineError> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let from = input.from_format.unwrap_or(eng::InputFormat::Cem);
        if !matches!(from, eng::InputFormat::Cem) {
            continue;
        }
        let bytes = if input.bytes.is_empty() {
            materialize_fixture_input(context, input)?.bytes
        } else {
            input.bytes.clone()
        };
        diagnostics.extend(template_pass::run(&bytes, from, Some(&input.uri)));
    }
    Ok(diagnostics)
}

fn materialize_fixture_input(
    context: &eng::EngineContext,
    input: &eng::EngineInput,
) -> Result<eng::EngineInput, EngineError> {
    if !input.bytes.is_empty() {
        return Ok(input.clone());
    }

    let path = fixture_materialized_input_path(&input.uri);
    let read = read_source(
        context,
        &path,
        "input URI",
        ResolvePurpose::Input,
        input.root_scope.default_content_type.as_deref(),
    )
    .map_err(|source| EngineError::Io {
        path: input.uri.clone().into(),
        source,
    })?;
    let mut root_scope = input.root_scope.clone();
    if root_scope.default_content_type.is_none() {
        root_scope.default_content_type = read.content_type;
    }
    Ok(eng::EngineInput {
        bytes: read.bytes,
        identity: root_scope.format_identity_option(),
        root_scope,
        ..input.clone()
    })
}

fn fixture_materialized_input_path(uri: &str) -> PathBuf {
    if uri.starts_with("file://") || (uri_scheme(uri).is_some() && !is_windows_drive_path(uri)) {
        return PathBuf::from(uri);
    }
    resolve_fixture_input_path(uri)
}

fn resolve_fixture_input_path(uri: &str) -> PathBuf {
    if uri.starts_with("file://") {
        return PathBuf::from(uri);
    }
    let path = Path::new(uri);
    if path.is_absolute() || path.exists() {
        path.to_path_buf()
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }
}

fn merge_embedding_diagnostics(
    report: &mut cem_ml::report::Report,
    embedding: Vec<cem_ml::diagnostics::Diagnostic>,
) {
    if embedding.is_empty() {
        return;
    }
    for diagnostic in &embedding {
        match diagnostic.severity {
            cem_ml::diagnostics::Severity::Info => report.summary.info_count += 1,
            cem_ml::diagnostics::Severity::Warning => report.summary.warning_count += 1,
            cem_ml::diagnostics::Severity::Error => report.summary.error_count += 1,
            cem_ml::diagnostics::Severity::Fatal => report.summary.fatal_count += 1,
        }
        if diagnostic.severity.is_hard_violation() {
            report.summary.hard_violation_count += 1;
        }
    }
    report.diagnostics.extend(embedding);
}

fn write_diagnostics(diags: &[cem_ml::diagnostics::Diagnostic], s: &mut Streams<'_>) {
    if s.quiet {
        return;
    }
    for d in diags {
        let _ = writeln!(
            s.stderr,
            "{}:{}:{}: {}: {} [{}]",
            d.uri.as_deref().unwrap_or("<unknown>"),
            d.line.unwrap_or(0),
            d.column.unwrap_or(0),
            severity_label(d.severity),
            d.message,
            d.code,
        );
    }
}

fn severity_label(s: cem_ml::diagnostics::Severity) -> &'static str {
    use cem_ml::diagnostics::Severity::*;
    match s {
        Info => "info",
        Warning => "warning",
        Error => "error",
        Fatal => "fatal",
    }
}

/// Default basenames per `cem-ml-cli-contract.md` §Report Ownership.
/// Files land under `packages/cem_ml_cli/dist/` when the user supplies that
/// directory; the basenames disambiguate the command that produced them.
pub const REPORT_BASENAME_VALIDATE: &str = "cem-ml.report";
pub const REPORT_BASENAME_ROUNDTRIP: &str = "cem-ml.roundtrip.report";
pub const REPORT_BASENAME_BENCH: &str = "cem-ml.bench.report";
pub const REPORT_BASENAME_CONVERT: &str = "cem-ml.convert.report";
pub const REPORT_BASENAME_TRANSFORM: &str = "cem-ml.transform.report";

fn resolve_report_target(p: &Path, basename: &str, ext: &str) -> std::path::PathBuf {
    if p.extension().is_some() {
        p.to_path_buf()
    } else {
        p.join(format!("{basename}.{ext}"))
    }
}

fn write_report_target(
    context: &eng::EngineContext,
    p: &Path,
    basename: &str,
    ext: &str,
    contents: &[u8],
) -> io::Result<()> {
    let raw_target = resolve_report_target(p, basename, ext);
    write_destination(
        context,
        &raw_target,
        "report destination",
        ResolvePurpose::Report,
        contents,
    )
}

fn write_report_files(
    context: &eng::EngineContext,
    report: &cem_ml::report::Report,
    report_opts: &cli::ReportOptions,
    basename: &str,
) -> io::Result<()> {
    if let Some(p) = &report_opts.report_json {
        let body = serde_json::to_string_pretty(report)?;
        write_report_target(context, p, basename, "json", body.as_bytes())?;
    }
    if let Some(p) = &report_opts.report_md {
        let body = render_report_markdown(report);
        write_report_target(context, p, basename, "md", body.as_bytes())?;
    }
    Ok(())
}

fn write_benchmark_report_files(
    context: &eng::EngineContext,
    body: &serde_json::Value,
    report_opts: &cli::ReportOptions,
) -> io::Result<()> {
    if let Some(p) = &report_opts.report_json {
        let body = serde_json::to_string_pretty(body)?;
        write_report_target(context, p, REPORT_BASENAME_BENCH, "json", body.as_bytes())?;
    }
    if let Some(p) = &report_opts.report_md {
        let body = render_benchmark_report_markdown(body);
        write_report_target(context, p, REPORT_BASENAME_BENCH, "md", body.as_bytes())?;
    }
    Ok(())
}

fn report_requested(report_opts: &cli::ReportOptions) -> bool {
    report_opts.report_json.is_some() || report_opts.report_md.is_some()
}

fn report_options_snapshot(
    fail_level: cli::FailLevel,
    context: &cli::ContextOptions,
) -> cem_ml::report::ReportOptionsSnapshot {
    cem_ml::report::ReportOptionsSnapshot {
        fail_level: to_engine_fail_level(fail_level),
        schema: context.schema.clone(),
        content_type: context.content_type.clone(),
        base_uri: context.base_uri.clone(),
    }
}

fn run_config_diagnostic_inputs(config: Option<Box<RunConfig>>) -> Vec<String> {
    config
        .map(|config| {
            let config = *config;
            config.inputs.into_iter().map(|input| input.uri).collect()
        })
        .unwrap_or_default()
}

fn handle_run_config_diagnostics_report(
    config: Option<Box<RunConfig>>,
    diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
    fail_level: cli::FailLevel,
    context: &cli::ContextOptions,
    report_opts: &cli::ReportOptions,
    basename: &str,
    s: &mut Streams<'_>,
) -> Outcome {
    let _ = writeln!(s.stderr, "cem-ml: invalid run config");
    write_diagnostics(&diagnostics, s);
    let report = cem_ml::report::Report::deterministic(
        run_config_diagnostic_inputs(config),
        diagnostics,
        report_options_snapshot(fail_level, context),
    );
    let engine_context = self::context(context);
    if let Err(e) = write_report_files(&engine_context, &report, report_opts, basename) {
        let _ = writeln!(s.stderr, "cem-ml: report write failure: {e}");
        return Outcome::code(EXIT_IO);
    }
    if !s.quiet {
        let json = serde_json::to_string_pretty(&report).unwrap_or_default();
        let _ = writeln!(s.stdout, "{json}");
    }
    Outcome::code(EXIT_USAGE_OR_RESERVED)
}

fn append_convert_scheduler_trace(
    combined: &mut cem_ml::report::SchedulerTraceReport,
    trace: &cem_ml::report::SchedulerTraceReport,
) {
    for event in &trace.events {
        let mut event = event.clone();
        event.sequence = combined.events.len() as u64;
        combined.events.push(event);
    }
    combined.event_count = combined.events.len() as u64;
}

fn write_convert_report_if_requested(
    context: &eng::EngineContext,
    args: &cli::ConvertArgs,
    input_uris: &[String],
    diagnostics: &[cem_ml::diagnostics::Diagnostic],
    scheduler_trace: &cem_ml::report::SchedulerTraceReport,
) -> io::Result<()> {
    if !report_requested(&args.report) {
        return Ok(());
    }
    let report = cem_ml::report::Report::deterministic(
        input_uris.to_vec(),
        diagnostics.to_vec(),
        report_options_snapshot(cli::FailLevel::Validate, &args.context),
    )
    .with_scheduler_trace_report(scheduler_trace.clone());
    write_report_files(context, &report, &args.report, REPORT_BASENAME_CONVERT)
}

fn write_transform_report_if_requested(
    context: &eng::EngineContext,
    args: &cli::TransformArgs,
    input_uris: &[String],
    diagnostics: &[cem_ml::diagnostics::Diagnostic],
    scheduler_trace: &cem_ml::report::SchedulerTraceReport,
    transform: Option<cem_ml::report::TransformReport>,
    transform_graph: Option<cem_ml::report::TransformGraphReport>,
) -> io::Result<()> {
    if !report_requested(&args.report) {
        return Ok(());
    }
    let mut report = cem_ml::report::Report::deterministic(
        input_uris.to_vec(),
        diagnostics.to_vec(),
        report_options_snapshot(cli::FailLevel::Validate, &args.context),
    )
    .with_scheduler_trace_report(scheduler_trace.clone());
    report.report_ast.transform = transform;
    report.report_ast.transform_graph = transform_graph;
    write_report_files(context, &report, &args.report, REPORT_BASENAME_TRANSFORM)
}

fn transform_graph_output_kind(primary: &serde_json::Value) -> String {
    match primary {
        serde_json::Value::String(_) => "document".to_owned(),
        serde_json::Value::Object(map) => map
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("object")
            .to_owned(),
        serde_json::Value::Array(_) => "array".to_owned(),
        serde_json::Value::Bool(_) => "boolean".to_owned(),
        serde_json::Value::Number(_) => "number".to_owned(),
        serde_json::Value::Null => "null".to_owned(),
    }
}

fn transform_graph_artifact_source_map_value(
    artifact: &eng::TransformGraphArtifact,
) -> Option<serde_json::Value> {
    artifact
        .source_map
        .as_ref()
        .and_then(|source_map| serde_json::to_value(source_map).ok())
        .or_else(|| {
            artifact
                .primary
                .get("sourceMap")
                .filter(|source_map| !source_map.is_null())
                .cloned()
        })
        .or_else(|| transform_graph_collection_source_map_value(&artifact.primary))
}

fn transform_graph_has_source_map(artifact: &eng::TransformGraphArtifact) -> bool {
    transform_graph_artifact_source_map_value(artifact).is_some()
}

fn transform_graph_output_span_count(artifact: &eng::TransformGraphArtifact) -> u64 {
    if artifact.source_map.is_some() || !artifact.output_spans.is_empty() {
        return artifact.output_spans.len() as u64;
    }
    artifact
        .primary
        .get("outputSpans")
        .and_then(serde_json::Value::as_array)
        .map(|spans| spans.len() as u64)
        .unwrap_or(0)
}

fn transform_graph_collection_source_map_value(
    primary: &serde_json::Value,
) -> Option<serde_json::Value> {
    if primary.get("kind").and_then(serde_json::Value::as_str) != Some("collection") {
        return None;
    }
    let items = primary.get("items")?.as_array()?;
    let items = items
        .iter()
        .filter_map(|item| {
            let source_map = item.get("sourceMap").filter(|source_map| !source_map.is_null())?;
            Some(serde_json::json!({
                "input": item.get("input").cloned().unwrap_or(serde_json::Value::Null),
                "artifactId": item.get("artifactId").cloned().unwrap_or(serde_json::Value::Null),
                "sourceMap": source_map.clone(),
                "outputSpans": item.get("outputSpans").cloned().unwrap_or_else(|| serde_json::json!([])),
            }))
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        Some(serde_json::json!({
            "kind": "collection",
            "items": items,
        }))
    }
}

fn transform_graph_collection_item_reports(
    primary: &serde_json::Value,
) -> Vec<cem_ml::report::TransformGraphCollectionItemReport> {
    if primary.get("kind").and_then(serde_json::Value::as_str) != Some("collection") {
        return Vec::new();
    }
    let Some(items) = primary.get("items").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| {
            let output_span_count = item
                .get("outputSpans")
                .and_then(serde_json::Value::as_array)
                .map(|spans| spans.len() as u64)
                .unwrap_or(0);
            cem_ml::report::TransformGraphCollectionItemReport {
                input: item
                    .get("input")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                artifact_id: item
                    .get("artifactId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                has_source_map: item
                    .get("sourceMap")
                    .is_some_and(|source_map| !source_map.is_null()),
                output_span_count,
            }
        })
        .collect()
}

fn transform_graph_source_map_ref(
    destination: Option<&str>,
    has_source_map: bool,
) -> Option<String> {
    if has_source_map {
        destination.map(|destination| format!("{destination}.map"))
    } else {
        None
    }
}

fn transform_report_from_response(
    response: &eng::TransformResponse,
    input: &str,
    destination: Option<&Path>,
) -> cem_ml::report::TransformReport {
    let has_source_map = response.source_map.is_some();
    let destination = destination.map(|path| path.display().to_string());
    cem_ml::report::TransformReport {
        input: input.to_owned(),
        destination: destination.clone(),
        output_kind: transform_graph_output_kind(&response.primary),
        has_source_map,
        output_span_count: response.output_spans.len() as u64,
        source_map_ref: transform_graph_source_map_ref(destination.as_deref(), has_source_map),
    }
}

fn transform_graph_report_from_artifacts(
    artifacts: &[eng::TransformGraphArtifact],
) -> cem_ml::report::TransformGraphReport {
    let exports = artifacts
        .iter()
        .map(|artifact| {
            let has_source_map = transform_graph_has_source_map(artifact);
            cem_ml::report::TransformGraphExportReport {
                export_id: artifact.export_id.clone(),
                input: artifact.input.clone(),
                destination: artifact.destination.clone(),
                content_type: artifact
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.content_type.clone()),
                schema: artifact
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.schema.clone()),
                output_kind: transform_graph_output_kind(&artifact.primary),
                has_source_map,
                output_span_count: transform_graph_output_span_count(artifact),
                source_map_ref: transform_graph_source_map_ref(
                    artifact.destination.as_deref(),
                    has_source_map,
                ),
                collection_items: transform_graph_collection_item_reports(&artifact.primary),
            }
        })
        .collect::<Vec<_>>();
    cem_ml::report::TransformGraphReport {
        export_count: exports.len() as u64,
        exports,
    }
}

fn render_report_markdown(report: &cem_ml::report::Report) -> String {
    let mut out = String::new();
    out.push_str("# cem-ml report\n\n");
    out.push_str(&format!("Generated: {}\n\n", report.generated_at));
    out.push_str(&format!("- inputs: {}\n", report.summary.input_count));
    out.push_str(&format!("- info: {}\n", report.summary.info_count));
    out.push_str(&format!("- warning: {}\n", report.summary.warning_count));
    out.push_str(&format!("- error: {}\n", report.summary.error_count));
    out.push_str(&format!("- fatal: {}\n", report.summary.fatal_count));
    out.push_str(&format!(
        "- hardViolations: {}\n",
        report.summary.hard_violation_count
    ));
    if let Some(transform) = &report.report_ast.transform {
        out.push_str("\n## transform\n\n");
        out.push_str(&format!(
            "- primary <- {} -> {} [{}]",
            transform.input,
            transform.destination.as_deref().unwrap_or("<stdout>"),
            transform.output_kind
        ));
        out.push_str(&format!(
            " [sourceMap: {}, outputSpans: {}]",
            if transform.has_source_map {
                "yes"
            } else {
                "no"
            },
            transform.output_span_count
        ));
        if let Some(source_map_ref) = transform.source_map_ref.as_deref() {
            out.push_str(&format!(" [sourceMapRef: {source_map_ref}]"));
        }
        out.push('\n');
    }
    if let Some(transform_graph) = &report.report_ast.transform_graph {
        out.push_str("\n## transform graph\n\n");
        out.push_str(&format!("- exports: {}\n", transform_graph.export_count));
        for export in &transform_graph.exports {
            out.push_str(&format!(
                "- {} <- {} -> {}",
                export.export_id,
                export.input,
                export.destination.as_deref().unwrap_or("<stdout>")
            ));
            if let Some(content_type) = export.content_type.as_deref() {
                out.push_str(&format!(" ({content_type})"));
            }
            out.push_str(&format!(
                " [sourceMap: {}, outputSpans: {}]",
                if export.has_source_map { "yes" } else { "no" },
                export.output_span_count
            ));
            if let Some(source_map_ref) = export.source_map_ref.as_deref() {
                out.push_str(&format!(" [sourceMapRef: {source_map_ref}]"));
            }
            if !export.collection_items.is_empty() {
                out.push_str(&format!(
                    " [collectionItems: {}]",
                    export.collection_items.len()
                ));
            }
            out.push('\n');
        }
    }
    out
}

fn render_benchmark_report_markdown(body: &serde_json::Value) -> String {
    let body = serde_json::to_string_pretty(body).unwrap_or_default();
    format!("# cem-ml benchmark report\n\n```json\n{body}\n```\n")
}

fn fail_for_summary(fail_level: cli::FailLevel, report: &cem_ml::report::Report) -> bool {
    let s = &report.summary;
    match fail_level {
        cli::FailLevel::Strict => {
            s.warning_count + s.error_count + s.fatal_count + s.info_count > 0
        }
        cli::FailLevel::Validate => s.error_count + s.fatal_count > 0,
        cli::FailLevel::Parse => s.fatal_count > 0,
    }
}

pub fn run_parse<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::ParseArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(CliRequestError::RunConfigDiagnostics {
            config,
            diagnostics,
        }) => {
            return handle_run_config_diagnostics_report(
                config,
                diagnostics,
                args.fail_level,
                &args.context,
                &args.report,
                REPORT_BASENAME_VALIDATE,
                s,
            );
        }
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let input = match single_configured_input(
        &engine_context,
        args.input.as_deref(),
        args.from_format,
        &config,
        &input_defaults,
    ) {
        Ok(i) => i,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let embedding_diags = template_pass::run(
        &input.bytes,
        input.from_format.unwrap_or(eng::InputFormat::Cem),
        Some(input.uri.as_str()),
    );
    let input_uri = input.uri.clone();
    let req = eng::ParseRequest {
        input,
        projection: to_engine_parse_projection(args.format),
        fail_level: to_engine_fail_level(args.fail_level),
        preserve_source_offsets: args.preserve_source_offsets,
        context: engine_context.clone(),
    };
    match engine.parse(req) {
        Ok(mut resp) => {
            if let Err(e) = write_primary(&engine_context, &resp.primary, args.out.as_deref(), s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            resp.diagnostics.extend(embedding_diags);
            write_diagnostics(&resp.diagnostics, s);
            if report_requested(&args.report) {
                let report = cem_ml::report::Report::deterministic(
                    vec![input_uri],
                    resp.diagnostics,
                    report_options_snapshot(args.fail_level, &args.context),
                );
                if let Err(e) = write_report_files(
                    &engine_context,
                    &report,
                    &args.report,
                    REPORT_BASENAME_VALIDATE,
                ) {
                    let _ = writeln!(s.stderr, "cem-ml: parse report write failure: {e}");
                    return Outcome::code(EXIT_IO);
                }
            }
            Outcome::ok()
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_validate<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::ValidateArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(CliRequestError::RunConfigDiagnostics {
            config,
            diagnostics,
        }) => {
            return handle_run_config_diagnostics_report(
                config,
                diagnostics,
                args.fail_level,
                &args.context,
                &args.report,
                REPORT_BASENAME_VALIDATE,
                s,
            );
        }
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let inputs = match collect_configured_inputs(
        &engine_context,
        &args.inputs,
        args.from_format,
        &config,
        &input_defaults,
    ) {
        Ok(v) => v,
        Err(err) => return handle_cli_request_error(err, s),
    };
    if inputs.is_empty() {
        return handle_cli_request_error(
            CliRequestError::Usage("validate requires at least one input or --input-spec".into()),
            s,
        );
    }
    if let Some(report) = direct_source_validation_report(&inputs, args.fail_level, &args.context) {
        if let Err(e) = write_report_files(
            &engine_context,
            &report,
            &args.report,
            REPORT_BASENAME_VALIDATE,
        ) {
            let _ = writeln!(s.stderr, "cem-ml: report write failure: {e}");
            return Outcome::code(EXIT_IO);
        }
        if !s.quiet {
            let json = serde_json::to_string_pretty(&report).unwrap_or_default();
            let _ = writeln!(s.stdout, "{json}");
        }
        if fail_for_summary(args.fail_level, &report) {
            return Outcome::code(EXIT_HARD_FAILURE);
        }
        return Outcome::ok();
    }
    let embedding_diags = collect_embedding_diagnostics(&inputs);
    let req = eng::ValidateRequest {
        inputs,
        projection: to_engine_validate_projection(args.format),
        fail_level: to_engine_fail_level(args.fail_level),
        context: engine_context.clone(),
    };
    match engine.validate(req) {
        Ok(mut resp) => {
            merge_embedding_diagnostics(&mut resp.report, embedding_diags);
            if let Err(e) = write_report_files(
                &engine_context,
                &resp.report,
                &args.report,
                REPORT_BASENAME_VALIDATE,
            ) {
                let _ = writeln!(s.stderr, "cem-ml: report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if !s.quiet {
                let json = serde_json::to_string_pretty(&resp.report).unwrap_or_default();
                let _ = writeln!(s.stdout, "{json}");
            }
            if fail_for_summary(args.fail_level, &resp.report) {
                Outcome::code(EXIT_HARD_FAILURE)
            } else {
                Outcome::ok()
            }
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_check<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::CheckArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(CliRequestError::RunConfigDiagnostics {
            config,
            diagnostics,
        }) => {
            return handle_run_config_diagnostics_report(
                config,
                diagnostics,
                args.fail_level,
                &args.context,
                &args.report,
                REPORT_BASENAME_VALIDATE,
                s,
            );
        }
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let inputs = match collect_configured_inputs(
        &engine_context,
        &args.inputs,
        args.from_format,
        &config,
        &input_defaults,
    ) {
        Ok(v) => v,
        Err(err) => return handle_cli_request_error(err, s),
    };
    if inputs.is_empty() {
        return handle_cli_request_error(
            CliRequestError::Usage("check requires at least one input or --input-spec".into()),
            s,
        );
    }
    if let Some(report) = direct_source_validation_report(&inputs, args.fail_level, &args.context) {
        if let Err(e) = write_report_files(
            &engine_context,
            &report,
            &args.report,
            REPORT_BASENAME_VALIDATE,
        ) {
            let _ = writeln!(s.stderr, "cem-ml: report write failure: {e}");
            return Outcome::code(EXIT_IO);
        }
        if !s.quiet {
            let json = serde_json::to_string_pretty(&report).unwrap_or_default();
            let _ = writeln!(s.stdout, "{json}");
        }
        if args.zero_hard_violations && report.summary.hard_violation_count > 0 {
            return Outcome::code(EXIT_HARD_FAILURE);
        }
        if fail_for_summary(args.fail_level, &report) {
            return Outcome::code(EXIT_HARD_FAILURE);
        }
        return Outcome::ok();
    }
    let embedding_diags = collect_embedding_diagnostics(&inputs);
    let req = eng::CheckRequest {
        inputs,
        projection: to_engine_validate_projection(args.format),
        fail_level: to_engine_fail_level(args.fail_level),
        zero_hard_violations: args.zero_hard_violations,
        context: engine_context.clone(),
    };
    match engine.check(req) {
        Ok(mut resp) => {
            merge_embedding_diagnostics(&mut resp.report, embedding_diags);
            resp.hard_violation_count = resp.report.summary.hard_violation_count;
            if let Err(e) = write_report_files(
                &engine_context,
                &resp.report,
                &args.report,
                REPORT_BASENAME_VALIDATE,
            ) {
                let _ = writeln!(s.stderr, "cem-ml: report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if !s.quiet {
                let json = serde_json::to_string_pretty(&resp.report).unwrap_or_default();
                let _ = writeln!(s.stdout, "{json}");
            }
            if args.zero_hard_violations && resp.hard_violation_count > 0 {
                return Outcome::code(EXIT_HARD_FAILURE);
            }
            if fail_for_summary(args.fail_level, &resp.report) {
                Outcome::code(EXIT_HARD_FAILURE)
            } else {
                Outcome::ok()
            }
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_inspect<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::InspectArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let input = match single_configured_input(
        &engine_context,
        args.input.as_deref(),
        args.from_format,
        &config,
        &input_defaults,
    ) {
        Ok(i) => i,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let req = eng::InspectRequest {
        input,
        show: to_engine_inspect_view(args.show),
        context: engine_context.clone(),
    };
    match engine.inspect(req) {
        Ok(resp) => {
            if let Err(e) = write_primary(&engine_context, &resp.body, args.out.as_deref(), s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            Outcome::ok()
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_convert<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::ConvertArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let output_defaults = output_scope_defaults(&args);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), output_defaults),
    ) {
        Ok(config) => config,
        Err(CliRequestError::RunConfigDiagnostics {
            config,
            diagnostics,
        }) => {
            return handle_run_config_diagnostics_report(
                config,
                diagnostics,
                cli::FailLevel::Validate,
                &args.context,
                &args.report,
                REPORT_BASENAME_CONVERT,
                s,
            );
        }
        Err(err) => return handle_cli_request_error(err, s),
    };
    if !config.outputs.is_empty() {
        return run_convert_fanout(engine, &args, &config, &input_defaults, s);
    }
    let engine_context = context_with_config(&args.context, &config);
    let input = match single_configured_input(
        &engine_context,
        args.input.as_deref(),
        args.from_format,
        &config,
        &input_defaults,
    ) {
        Ok(i) => i,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let input_uri = input.uri.clone();
    let req = eng::ConvertRequest {
        input,
        to_format: to_engine_layer_format(args.to_format),
        preserve_source_offsets: args.preserve_source_offsets,
        context: engine_context.clone(),
        target: convert_target_identity_with_config(&args, &config),
        target_scope: convert_target_scope_with_config(&args, &config),
        scheduler_scope_id: 0,
    };
    match engine.convert(req) {
        Ok(resp) => {
            let out = convert_output_destination(&args, &config);
            if let Err(e) = write_convert_primary(&engine_context, &resp, out.as_deref(), s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            write_diagnostics(&resp.diagnostics, s);
            if let Err(e) = write_convert_report_if_requested(
                &engine_context,
                &args,
                &[input_uri],
                &resp.diagnostics,
                &resp.scheduler_trace,
            ) {
                let _ = writeln!(s.stderr, "cem-ml: convert report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            Outcome::ok()
        }
        Err(e) => handle_engine_error(e, s),
    }
}

fn write_transform_graph_artifacts(
    context: &eng::EngineContext,
    artifacts: &[eng::TransformGraphArtifact],
    s: &mut Streams<'_>,
) -> io::Result<()> {
    for artifact in artifacts {
        let out = artifact.destination.as_deref().map(Path::new);
        write_document_primary(context, &artifact.primary, out, s)?;
        write_transform_graph_source_map_sidecar(context, artifact)?;
    }
    Ok(())
}

fn write_transform_source_map_sidecar(
    context: &eng::EngineContext,
    response: &eng::TransformResponse,
    destination: Option<&Path>,
    input: &str,
) -> io::Result<()> {
    let Some(destination) = destination else {
        return Ok(());
    };
    let Some(source_map) = response
        .source_map
        .as_ref()
        .and_then(|source_map| serde_json::to_value(source_map).ok())
    else {
        return Ok(());
    };
    let destination = destination.to_string_lossy();
    let Some(source_map_ref) = transform_graph_source_map_ref(Some(destination.as_ref()), true)
    else {
        return Ok(());
    };
    let sidecar =
        transform_graph_source_map_sidecar_payload(&source_map, "primary", input, &destination);
    let bytes = serde_json::to_vec_pretty(&sidecar)?;
    write_destination(
        context,
        Path::new(&source_map_ref),
        "source-map sidecar destination",
        ResolvePurpose::Output,
        &bytes,
    )
}

fn write_transform_graph_source_map_sidecar(
    context: &eng::EngineContext,
    artifact: &eng::TransformGraphArtifact,
) -> io::Result<()> {
    let Some(destination) = artifact.destination.as_deref() else {
        return Ok(());
    };
    let Some(source_map) = transform_graph_artifact_source_map_value(artifact) else {
        return Ok(());
    };

    let Some(source_map_ref) = transform_graph_source_map_ref(Some(destination), true) else {
        return Ok(());
    };
    let sidecar = transform_graph_source_map_sidecar_payload(
        &source_map,
        artifact.export_id.as_str(),
        artifact.input.as_str(),
        destination,
    );
    let bytes = serde_json::to_vec_pretty(&sidecar)?;
    write_destination(
        context,
        Path::new(&source_map_ref),
        "source-map sidecar destination",
        ResolvePurpose::Output,
        &bytes,
    )
}

fn transform_graph_source_map_sidecar_payload(
    source_map: &serde_json::Value,
    export_id: &str,
    input: &str,
    destination: &str,
) -> serde_json::Value {
    let mut sidecar = source_map.clone();
    if let serde_json::Value::Object(fields) = &mut sidecar {
        fields.insert(
            "exportId".to_owned(),
            serde_json::Value::String(export_id.to_owned()),
        );
        fields.insert(
            "input".to_owned(),
            serde_json::Value::String(input.to_owned()),
        );
        fields.insert(
            "destination".to_owned(),
            serde_json::Value::String(destination.to_owned()),
        );
    }
    sidecar
}

fn run_transform_graph<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: &cli::TransformArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let engine_context = context(&args.context);
    let (req, config_source_uri) = match transform_graph_request_from_args(&engine_context, args) {
        Ok(req) => req,
        Err(err) => return handle_cli_request_error(err, s),
    };
    match engine.transform_graph(req) {
        Ok(resp) => {
            let requested_report = report_requested(&args.report);
            if let Err(e) = write_transform_report_if_requested(
                &engine_context,
                args,
                &[config_source_uri],
                &resp.diagnostics,
                &resp.scheduler_trace,
                None,
                Some(transform_graph_report_from_artifacts(&resp.artifacts)),
            ) {
                let _ = writeln!(s.stderr, "cem-ml: transform report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if !requested_report {
                write_diagnostics(&resp.diagnostics, s);
            }
            if resp
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity.is_hard_violation())
            {
                return Outcome::code(EXIT_HARD_FAILURE);
            }
            if let Err(e) = write_transform_graph_artifacts(&engine_context, &resp.artifacts, s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            Outcome::ok()
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_transform<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::TransformArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    if args.config.is_some() {
        return run_transform_graph(engine, &args, s);
    }
    let engine_context = context(&args.context);
    let req = match transform_request_from_args(&engine_context, &args) {
        Ok(req) => req,
        Err(err) => return handle_cli_request_error(err, s),
    };
    match engine.transform(req) {
        Ok(resp) => {
            let report_requested = report_requested(&args.report);
            let input = args
                .data
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            if let Err(e) = write_transform_report_if_requested(
                &engine_context,
                &args,
                std::slice::from_ref(&input),
                &resp.diagnostics,
                &resp.scheduler_trace,
                Some(transform_report_from_response(
                    &resp,
                    &input,
                    args.out.as_deref(),
                )),
                None,
            ) {
                let _ = writeln!(s.stderr, "cem-ml: transform report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if !report_requested {
                write_diagnostics(&resp.diagnostics, s);
            }
            if resp
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity.is_hard_violation())
            {
                return Outcome::code(EXIT_HARD_FAILURE);
            }
            if let Err(e) =
                write_document_primary(&engine_context, &resp.primary, args.out.as_deref(), s)
            {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if let Err(e) = write_transform_source_map_sidecar(
                &engine_context,
                &resp,
                args.out.as_deref(),
                &input,
            ) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            Outcome::ok()
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_trace<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::TraceArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let input = match single_configured_input(
        &engine_context,
        args.input.as_deref(),
        args.from_format,
        &config,
        &input_defaults,
    ) {
        Ok(i) => i,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let req = eng::TraceRequest {
        input,
        projection: to_engine_trace_projection(args.format),
        context: engine_context.clone(),
    };
    match engine.trace(req) {
        Ok(resp) => {
            if let Err(e) = write_primary(&engine_context, &resp.body, args.out.as_deref(), s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            Outcome::ok()
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_bench<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::BenchArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(CliRequestError::RunConfigDiagnostics {
            config,
            diagnostics,
        }) => {
            return handle_run_config_diagnostics_report(
                config,
                diagnostics,
                cli::FailLevel::Validate,
                &args.context,
                &args.report,
                REPORT_BASENAME_BENCH,
                s,
            );
        }
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let inputs = match collect_configured_inputs(
        &engine_context,
        &args.inputs,
        None,
        &config,
        &input_defaults,
    ) {
        Ok(v) => v,
        Err(err) => return handle_cli_request_error(err, s),
    };
    if inputs.is_empty() {
        return handle_cli_request_error(
            CliRequestError::Usage("bench requires at least one input or --input-spec".into()),
            s,
        );
    }
    let req = eng::BenchRequest {
        inputs,
        projection: to_engine_bench_projection(args.format),
        iterations: args.iterations,
        budget_ms: args.budget_ms,
        profile: args.profile.map(to_engine_bench_profile),
        cold_cache: args.cold_cache,
        context: engine_context.clone(),
    };
    match engine.bench(req) {
        Ok(resp) => {
            if !s.quiet {
                let json = serde_json::to_string_pretty(&resp.body).unwrap_or_default();
                let _ = writeln!(s.stdout, "{json}");
            }
            if let Err(e) = write_benchmark_report_files(&engine_context, &resp.body, &args.report)
            {
                let _ = writeln!(s.stderr, "cem-ml: benchmark report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if resp.budget_exceeded {
                Outcome::code(EXIT_HARD_FAILURE)
            } else {
                Outcome::ok()
            }
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_fixture_validate<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::FixtureValidateArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(CliRequestError::RunConfigDiagnostics {
            config,
            diagnostics,
        }) => {
            return handle_run_config_diagnostics_report(
                config,
                diagnostics,
                args.fail_level,
                &args.context,
                &args.report,
                REPORT_BASENAME_VALIDATE,
                s,
            );
        }
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let mut inputs =
        collect_fixture_inputs(&args.inputs, config.inputs.is_empty(), &input_defaults);
    for spec in &config.inputs {
        match engine_input_from_spec(&engine_context, spec, None) {
            Ok(input) => inputs.push(input),
            Err(err) => return handle_cli_request_error(err, s),
        }
    }
    let embedding_diags = match collect_fixture_embedding_diagnostics(&engine_context, &inputs) {
        Ok(v) => v,
        Err(e) => return handle_engine_error(e, s),
    };
    let req = eng::FixtureValidateRequest {
        inputs,
        fail_level: to_engine_fail_level(args.fail_level),
        zero_hard_violations: args.zero_hard_violations,
        context: engine_context.clone(),
    };
    match engine.fixture_validate(req) {
        Ok(mut resp) => {
            merge_embedding_diagnostics(&mut resp.report, embedding_diags);
            if let Err(e) = write_report_files(
                &engine_context,
                &resp.report,
                &args.report,
                REPORT_BASENAME_VALIDATE,
            ) {
                let _ = writeln!(s.stderr, "cem-ml: report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if !s.quiet {
                let json = serde_json::to_string_pretty(&resp.report).unwrap_or_default();
                let _ = writeln!(s.stdout, "{json}");
            }
            if args.zero_hard_violations && resp.report.summary.hard_violation_count > 0 {
                return Outcome::code(EXIT_HARD_FAILURE);
            }
            if fail_for_summary(args.fail_level, &resp.report) {
                Outcome::code(EXIT_HARD_FAILURE)
            } else {
                Outcome::ok()
            }
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_fixture_roundtrip<E: CemMlEngine + ?Sized>(
    engine: &E,
    args: cli::FixtureRoundtripArgs,
    s: &mut Streams<'_>,
) -> Outcome {
    let input_defaults = input_scope_defaults(&args.context);
    let config = match run_config_for_context(
        &args.context,
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(CliRequestError::RunConfigDiagnostics {
            config,
            diagnostics,
        }) => {
            return handle_run_config_diagnostics_report(
                config,
                diagnostics,
                cli::FailLevel::Validate,
                &args.context,
                &args.report,
                REPORT_BASENAME_ROUNDTRIP,
                s,
            );
        }
        Err(err) => return handle_cli_request_error(err, s),
    };
    let engine_context = context_with_config(&args.context, &config);
    let mut inputs =
        collect_fixture_inputs(&args.inputs, config.inputs.is_empty(), &input_defaults);
    for spec in &config.inputs {
        match engine_input_from_spec(&engine_context, spec, None) {
            Ok(input) => inputs.push(input),
            Err(err) => return handle_cli_request_error(err, s),
        }
    }
    let req = eng::FixtureRoundtripRequest {
        inputs,
        to_format: to_engine_layer_format(args.to_format),
        context: engine_context.clone(),
    };
    match engine.fixture_roundtrip(req) {
        Ok(resp) => {
            if let Err(e) = write_report_files(
                &engine_context,
                &resp.report,
                &args.report,
                REPORT_BASENAME_ROUNDTRIP,
            ) {
                let _ = writeln!(s.stderr, "cem-ml: report write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            if !s.quiet {
                let body = serde_json::json!({
                    "report": resp.report,
                    "artifacts": resp.artifacts,
                });
                let _ = writeln!(
                    s.stdout,
                    "{}",
                    serde_json::to_string_pretty(&body).unwrap_or_default()
                );
            }
            Outcome::ok()
        }
        Err(e) => handle_engine_error(e, s),
    }
}

pub fn run_version(s: &mut Streams<'_>) -> Outcome {
    let _ = writeln!(s.stdout, "cem-ml {}", cem_ml::VERSION);
    let _ = writeln!(s.stdout, "{}", cli::COPYRIGHT_NOTICE);
    Outcome::ok()
}

pub fn run_reserved(name: &str, s: &mut Streams<'_>) -> Outcome {
    let _ = writeln!(
        s.stderr,
        "cem-ml: `{name}` is reserved and not yet implemented (exit 2 per cem-ml-cli-contract.md)."
    );
    Outcome::code(EXIT_USAGE_OR_RESERVED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::engine::NotImplementedEngine;
    use cem_ml::fake::FakeEngine;
    use cem_ml::real::RealCemMlEngine;
    use cem_ml::resolver::{ResolvedRead, ResolvedWrite, ResolverRegistry, ResourceResolver};
    use clap::Parser;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn test_source_map(len: u32) -> cem_ml::source_map::SourceMapStack {
        cem_ml::source_map::SourceMapStack {
            frames: vec![cem_ml::source_map::SourceMapFrame {
                source_id: cem_ml::source::SourceId(1),
                span: cem_ml::source_map::FrameSpan::Single(cem_ml::source::ByteRange::new(0, len)),
                transform: cem_ml::source_map::TransformKind::InterpreterRender,
            }],
        }
    }

    fn test_output_span(len: u32) -> cem_ml::interpreter::OutputSpan {
        cem_ml::interpreter::OutputSpan {
            output_range: cem_ml::source::ByteRange::new(0, len),
            origin: test_source_map(len),
        }
    }

    #[derive(Debug, Clone)]
    struct RecordingWriteResolver {
        writes: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    }

    #[derive(Debug, Clone)]
    struct StaticReadResolver {
        uri: &'static str,
        bytes: &'static [u8],
        content_type: Option<&'static str>,
    }

    impl ResourceResolver for StaticReadResolver {
        fn read(&self, _request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            Ok(ResolvedRead {
                uri: self.uri.to_owned(),
                bytes: self.bytes.to_vec(),
                content_type: self.content_type.map(str::to_owned),
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            _bytes: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
            Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            })
        }
    }

    impl ResourceResolver for RecordingWriteResolver {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Read,
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            bytes: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
            self.writes
                .lock()
                .unwrap()
                .push((request.uri.clone(), bytes.to_vec()));
            Ok(ResolvedWrite {
                uri: request.uri.clone(),
            })
        }
    }

    fn parse_cli(args: &[&str]) -> cli::Cli {
        cli::Cli::try_parse_from(std::iter::once("cem-ml").chain(args.iter().copied())).unwrap()
    }

    fn context_with_write_resolver(
        purpose: ResolvePurpose,
    ) -> (eng::EngineContext, Arc<Mutex<Vec<(String, Vec<u8>)>>>) {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+vfs",
            purpose,
            ResolveDirection::Write,
            RecordingWriteResolver {
                writes: writes.clone(),
            },
        );
        (
            eng::EngineContext {
                resolver_registry,
                ..eng::EngineContext::default()
            },
            writes,
        )
    }

    fn context_with_read_resolver(
        purpose: ResolvePurpose,
        resolver: impl ResourceResolver + 'static,
    ) -> eng::EngineContext {
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register("cem+vfs", purpose, ResolveDirection::Read, resolver);
        eng::EngineContext {
            resolver_registry,
            ..eng::EngineContext::default()
        }
    }

    fn run<E: CemMlEngine + ?Sized>(engine: &E, args: &[&str]) -> (Outcome, String, String) {
        let parsed = parse_cli(args);
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let quiet = parsed.quiet;
        let outcome = {
            let mut s = Streams {
                stdout: &mut stdout,
                stderr: &mut stderr,
                quiet,
            };
            dispatch(engine, parsed, &mut s)
        };
        (
            outcome,
            String::from_utf8(stdout.into_inner()).unwrap(),
            String::from_utf8(stderr.into_inner()).unwrap(),
        )
    }

    fn write_fixture(name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("cem-ml-cli-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    fn write_binary_fixture(name: &str, body: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join("cem-ml-cli-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    fn test_cem_document(input: &str) -> cem_ml::parser::document::CemDocument {
        let source = cem_ml::source::BytesSource::new(
            cem_ml::source::SourceId(1),
            input.as_bytes().to_vec(),
        );
        let tokenizer = cem_ml::tokenizer::cem::CemTokenizer::from_source(source);
        let normalizer = cem_ml::events::cem::CemEventNormalizer::new(tokenizer);
        cem_ml::parser::builder::CemAstBuilder::new(normalizer).build()
    }

    fn local_file_uri(path: &Path) -> String {
        format!("file://{}", path.display())
    }

    fn localhost_file_uri(path: &Path) -> String {
        format!("file://localhost{}", path.display())
    }

    fn assert_stderr_contains_all(stderr: &str, expected: &[&str]) {
        for needle in expected {
            assert!(
                stderr.contains(needle),
                "stderr did not contain `{needle}`:\n{stderr}"
            );
        }
    }

    fn assert_remote_resolver_boundary(stderr: &str, uri: &str) {
        assert_stderr_contains_all(
            stderr,
            &["remote/custom URI resolvers are not implemented", uri],
        );
    }

    fn assert_remote_input_resolver_boundary(stderr: &str, uri: &str) {
        assert_stderr_contains_all(
            stderr,
            &["remote/custom", "URI resolvers are not implemented", uri],
        );
    }

    fn assert_local_file_uri_boundary(stderr: &str, uri: &str) {
        assert_stderr_contains_all(stderr, &["only local file:// URIs are supported", uri]);
    }

    fn assert_remote_input_uri_rejected(args: &[&str], uri: &str) {
        let (outcome, stdout, stderr) = run(&FakeEngine, args);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["I/O error"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    fn assert_non_local_file_uri_input_rejected(args: &[&str], uri: &str) {
        let (outcome, stdout, stderr) = run(&FakeEngine, args);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["I/O error"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    fn assert_non_local_file_uri_output_rejected(args: &[&str], uri: &str) {
        let (outcome, stdout, stderr) = run(&FakeEngine, args);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    fn assert_remote_output_uri_rejected(args: &[&str], uri: &str) {
        let (outcome, stdout, stderr) = run(&FakeEngine, args);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn version_subcommand_prints_version_and_exits_zero() {
        let (outcome, stdout, _) = run(&NotImplementedEngine, &["version"]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.starts_with("cem-ml "));
        assert!(stdout.contains(cli::COPYRIGHT_NOTICE));
    }

    #[test]
    fn transform_writes_document_to_stdout_by_default() {
        let data = write_fixture("transform-run-data.cem", "{p @id=\"source\"}");
        let template = write_fixture(
            "transform-run-template.cem",
            "{section | {$datadom.attributes.kind}}",
        );

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--to-content-type",
                "text/html",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert_eq!(stdout, "<section>document</section>");
        assert!(stderr.trim().is_empty(), "{stderr}");
    }

    #[test]
    fn transform_writes_document_to_out_when_requested() {
        let data = write_fixture("transform-run-out-data.cem", "{p @id=\"source\"}");
        let template = write_fixture("transform-run-out-template.cem", "{section | Done}");
        let out = std::env::temp_dir().join("cem-ml-cli-tests/transform-out.html");
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(format!("{}.map", out.display()));

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--to-content-type",
                "text/html",
                "--out",
                out.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty());
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "<section>Done</section>"
        );
        let source_map: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(format!("{}.map", out.display())).unwrap(),
        )
        .unwrap();
        assert!(source_map["frames"].is_array());
        assert_eq!(source_map["exportId"], "primary");
        assert_eq!(source_map["input"], data.display().to_string());
        assert_eq!(source_map["destination"], out.display().to_string());
    }

    #[test]
    fn transform_config_executes_branched_cem_native_exports() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-run");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("data.cem"), "{p @id=\"source\"}").unwrap();
        std::fs::write(
            root.join("html.cem"),
            "{article | {$datadom.attributes.kind}}",
        )
        .unwrap();
        std::fs::write(root.join("chart.cem"), "{svg | {$datadom.attributes.kind}}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="data.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="html.cem" @template-content-type="text/cem-ml" |
      {export @id=main @out="out/book.html" @content-type="text/html"}
    }
    {transform @id=chart @src="chart.cem" @template-content-type="text/cem-ml" |
      {export @id=chart-out @out="out/book/chart.svg" @content-type="image/svg+xml"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/book.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/book/chart.svg")).unwrap(),
            "<svg>document</svg>"
        );
    }

    #[test]
    fn transform_config_glob_exports_apply_source_bindings() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-bindings");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("inputs")).unwrap();
        std::fs::write(root.join("inputs/ch01.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(root.join("inputs/ch02.cem"), "{p @id=\"two\"}").unwrap();
        std::fs::write(
            root.join("page.cem"),
            "{article | {$datadom.attributes.kind}}",
        )
        .unwrap();
        std::fs::write(root.join("chart.cem"), "{svg | {$datadom.attributes.kind}}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="inputs/*.cem" @content-type="text/cem-ml" |
    {transform @id=page @src="page.cem" @template-content-type="text/cem-ml" |
      {export @id=html @out="out/{stem}.html" @content-type="text/html"}
    }
    {transform @id=chart @src="chart.cem" @template-content-type="text/cem-ml" |
      {export @id=svg @out="out/{stem}/img/chart-{index}.{ext}.svg" @content-type="image/svg+xml"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/ch01.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/ch02.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/ch01/img/chart-0.cem.svg")).unwrap(),
            "<svg>document</svg>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/ch02/img/chart-1.cem.svg")).unwrap(),
            "<svg>document</svg>"
        );
    }

    #[test]
    fn transform_config_rewrites_html_importmap_for_dist() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-importmap");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("maps")).unwrap();
        let html = root.join("src/page.html");
        let source_map = root.join("maps/source.importmap.json");
        let target_map = root.join("maps/dist.importmap.json");
        let out = root.join("dist/page.html");
        let config = root.join("rewrite.cem");
        std::fs::write(
            &html,
            r#"<!doctype html>
<html>
  <head>
    <script type="importmap">
      {
        "imports": {
          "@pkg/": "../node_modules/@pkg/"
        }
      }
    </script>
  </head>
  <body></body>
</html>
"#,
        )
        .unwrap();
        std::fs::write(
            &source_map,
            r#"{"imports":{"@pkg/":"../node_modules/@pkg/"}}"#,
        )
        .unwrap();
        std::fs::write(&target_map, r#"{"imports":{"@pkg/":"./vendor/@pkg/"}}"#).unwrap();
        std::fs::write(
            &config,
            format!(
                "{{@doc cem-ml 1}}
{{run |
  {{import @id=page @src=\"{}\" @content-type=\"text/html\" |
    {{rewrite-importmap @id=imports @source-map=\"{}\" @target-map=\"{}\" |
      {{export @id=html @out=\"{}\" @content-type=\"text/html\"}}
    }}
  }}
}}
",
                html.display(),
                source_map.display(),
                target_map.display(),
                out.display()
            ),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["--quiet", "transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let written = std::fs::read_to_string(out).unwrap();
        assert!(written.contains("\"@pkg/\": \"./vendor/@pkg/\""));
        assert!(!written.contains("node_modules"));
        assert!(written.contains("<body></body>"));
    }

    #[test]
    fn transform_config_recursive_glob_exports_apply_source_bindings() {
        let root =
            std::env::temp_dir().join("cem-ml-cli-tests/transform-config-recursive-bindings");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("inputs/part-a")).unwrap();
        std::fs::create_dir_all(root.join("inputs/part-b/nested")).unwrap();
        std::fs::write(root.join("inputs/ch00.cem"), "{p @id=\"zero\"}").unwrap();
        std::fs::write(root.join("inputs/part-a/ch01.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(
            root.join("inputs/part-b/nested/ch02.cem"),
            "{p @id=\"two\"}",
        )
        .unwrap();
        std::fs::write(root.join("inputs/part-b/nested/skip.txt"), "skip").unwrap();
        std::fs::write(
            root.join("page.cem"),
            "{article | {$datadom.attributes.kind}}",
        )
        .unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="inputs/**/*.cem" @content-type="text/cem-ml" |
    {transform @id=page @src="page.cem" @template-content-type="text/cem-ml" |
      {export @id=html @out="out/{dir}/{stem}-{index}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/inputs/ch00-0.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/inputs/part-a/ch01-1.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/inputs/part-b/nested/ch02-2.html")).unwrap(),
            "<article>document</article>"
        );
        assert!(!root.join("out/inputs/part-b/nested/skip-3.html").exists());
    }

    #[test]
    fn transform_config_resolver_glob_exports_apply_source_bindings() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-resolver-glob");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("mirror/inputs")).unwrap();
        std::fs::write(root.join("mirror/inputs/ch02.cem"), "{p @id=\"two\"}").unwrap();
        std::fs::write(root.join("mirror/inputs/ch01.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(
            root.join("page.cem"),
            "{article | {$datadom.attributes.kind}}",
        )
        .unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="cem+vfs://workspace/inputs/*.cem" @content-type="text/cem-ml" |
    {transform @id=page @src="page.cem" @template-content-type="text/cem-ml" |
      {export @id=html @out="out/{index}-{stem}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                "--config",
                config.to_str().unwrap(),
                "--resolver-read-map",
                &format!("cem+vfs://workspace={}", root.join("mirror").display()),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/0-ch01.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/1-ch02.html")).unwrap(),
            "<article>document</article>"
        );
    }

    #[test]
    fn transform_config_resolver_recursive_glob_exports_apply_source_bindings() {
        let root =
            std::env::temp_dir().join("cem-ml-cli-tests/transform-config-resolver-recursive-glob");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("mirror/inputs/part-a")).unwrap();
        std::fs::create_dir_all(root.join("mirror/inputs/part-b/nested")).unwrap();
        std::fs::write(root.join("mirror/inputs/ch00.cem"), "{p @id=\"zero\"}").unwrap();
        std::fs::write(
            root.join("mirror/inputs/part-a/ch01.cem"),
            "{p @id=\"one\"}",
        )
        .unwrap();
        std::fs::write(
            root.join("mirror/inputs/part-b/nested/ch02.cem"),
            "{p @id=\"two\"}",
        )
        .unwrap();
        std::fs::write(
            root.join("page.cem"),
            "{article | {$datadom.attributes.kind}}",
        )
        .unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="cem+vfs://workspace/inputs/**/*.cem" @content-type="text/cem-ml" |
    {transform @id=page @src="page.cem" @template-content-type="text/cem-ml" |
      {export @id=html @out="out/{index}-{stem}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                "--config",
                config.to_str().unwrap(),
                "--resolver-read-map",
                &format!("cem+vfs://workspace={}", root.join("mirror").display()),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/0-ch00.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/1-ch01.html")).unwrap(),
            "<article>document</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/2-ch02.html")).unwrap(),
            "<article>document</article>"
        );
    }

    #[test]
    fn transform_config_collect_join_feeds_single_transform() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-collect-join");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("inputs")).unwrap();
        std::fs::write(root.join("inputs/ch02.cem"), "{p @id=\"two\"}").unwrap();
        std::fs::write(root.join("inputs/ch01.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(root.join("summary.cem"), "{article | {$input.count}}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=chapters @src="inputs/*.cem" @content-type="text/cem-ml" |
    {join @id=book @mode="collect" |
      {transform @id=summary @src="summary.cem" @template-content-type="text/cem-ml" |
        {export @id=html @out="out/book-{count}.html" @content-type="text/html"}
      }
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/book-2.html")).unwrap(),
            "<article>2</article>"
        );
        assert!(!root.join("out/book-0.html").exists());
        assert!(!root.join("out/book-1.html").exists());
    }

    #[test]
    fn transform_config_group_by_join_feeds_grouped_transforms() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-group-by-join");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("inputs/part-a")).unwrap();
        std::fs::create_dir_all(root.join("inputs/part-b")).unwrap();
        std::fs::write(root.join("inputs/part-a/ch01.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(root.join("inputs/part-a/ch02.cem"), "{p @id=\"two\"}").unwrap();
        std::fs::write(root.join("inputs/part-b/ch03.cem"), "{p @id=\"three\"}").unwrap();
        std::fs::write(root.join("summary.cem"), "{article | {$input.count}}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=chapters @src="inputs/**/*.cem" @content-type="text/cem-ml" |
    {join @id=section @mode="group-by" @by="dir" |
      {transform @id=summary @src="summary.cem" @template-content-type="text/cem-ml" |
        {export @id=html @out="out/{dir}/summary-{count}.html" @content-type="text/html"}
      }
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/inputs/part-a/summary-2.html")).unwrap(),
            "<article>2</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/inputs/part-b/summary-1.html")).unwrap(),
            "<article>1</article>"
        );
        assert!(!root.join("out/inputs/part-a/summary-0.html").exists());
    }

    #[test]
    fn transform_config_match_by_join_feeds_keyed_transforms() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-match-by-join");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("orders")).unwrap();
        std::fs::create_dir_all(root.join("customers")).unwrap();
        std::fs::write(root.join("orders/alice.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(root.join("orders/bob.cem"), "{p @id=\"two\"}").unwrap();
        std::fs::write(root.join("customers/alice.cem"), "{p @id=\"alice\"}").unwrap();
        std::fs::write(root.join("summary.cem"), "{article | {$input.count}}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=customers @src="customers/*.cem" @content-type="text/cem-ml"}
  {import @id=orders @src="orders/*.cem" @content-type="text/cem-ml" |
    {join @id=report @mode="match-by" @by="stem" @with:customers=customers |
      {transform @id=summary @src="summary.cem" @template-content-type="text/cem-ml" |
        {export @id=html @out="out/{stem}-{count}.html" @content-type="text/html"}
      }
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/alice-2.html")).unwrap(),
            "<article>2</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/bob-1.html")).unwrap(),
            "<article>1</article>"
        );
        assert!(!root.join("out/bob-2.html").exists());
    }

    #[test]
    fn transform_config_zip_join_feeds_positional_transforms() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-zip-join");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("pages")).unwrap();
        std::fs::create_dir_all(root.join("metadata")).unwrap();
        std::fs::write(root.join("pages/001.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(root.join("pages/002.cem"), "{p @id=\"two\"}").unwrap();
        std::fs::write(root.join("metadata/001.cem"), "{p @id=\"meta-one\"}").unwrap();
        std::fs::write(root.join("metadata/002.cem"), "{p @id=\"meta-two\"}").unwrap();
        std::fs::write(root.join("summary.cem"), "{article | {$input.count}}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=metadata @src="metadata/*.cem" @content-type="text/cem-ml"}
  {import @id=pages @src="pages/*.cem" @content-type="text/cem-ml" |
    {join @id=page @mode="zip" @with:metadata=metadata |
      {transform @id=summary @src="summary.cem" @template-content-type="text/cem-ml" |
        {export @id=html @out="out/{index}-{count}.html" @content-type="text/html"}
      }
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("out/0-2.html")).unwrap(),
            "<article>2</article>"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("out/1-2.html")).unwrap(),
            "<article>2</article>"
        );
        assert!(!root.join("out/2-2.html").exists());
    }

    #[test]
    fn transform_config_zip_join_rejects_count_mismatch() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-zip-join-mismatch");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("pages")).unwrap();
        std::fs::create_dir_all(root.join("metadata")).unwrap();
        std::fs::write(root.join("pages/001.cem"), "{p @id=\"one\"}").unwrap();
        std::fs::write(root.join("pages/002.cem"), "{p @id=\"two\"}").unwrap();
        std::fs::write(root.join("metadata/001.cem"), "{p @id=\"meta-one\"}").unwrap();
        std::fs::write(root.join("summary.cem"), "{article | {$input.count}}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=metadata @src="metadata/*.cem" @content-type="text/cem-ml"}
  {import @id=pages @src="pages/*.cem" @content-type="text/cem-ml" |
    {join @id=page @mode="zip" @with:metadata=metadata |
      {transform @id=summary @src="summary.cem" @template-content-type="text/cem-ml" |
        {export @id=html @out="out/{index}-{count}.html" @content-type="text/html"}
      }
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(
            stderr.contains("cem.transform_config.join_zip_count_mismatch"),
            "{stderr}"
        );
        assert!(!root.join("out/0-2.html").exists());
    }

    #[test]
    fn transform_config_report_records_graph_exports() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-report");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("data.cem"), "{p @id=\"source\"}").unwrap();
        std::fs::write(
            root.join("html.cem"),
            "{article | {$datadom.attributes.kind}}",
        )
        .unwrap();
        std::fs::write(root.join("svg.cem"), "{svg | {$datadom.attributes.kind}}").unwrap();
        let config = root.join("graph.cem");
        let report = root.join("report.json");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="data.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="html.cem" @template-content-type="text/cem-ml" |
      {export @id=main @out="out/{stem}.html" @content-type="text/html"}
    }
    {transform @id=svg @src="svg.cem" @template-content-type="text/cem-ml" |
      {export @id=chart @out="out/{stem}.svg" @content-type="image/svg+xml"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                "--config",
                config.to_str().unwrap(),
                "--report-json",
                report.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report).unwrap()).unwrap();
        assert_eq!(report["summary"]["inputCount"], 1);
        assert_eq!(report["reportAst"]["transformGraph"]["exportCount"], 2);
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][0]["exportId"],
            "main"
        );
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][0]["input"],
            "html"
        );
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][0]["destination"],
            root.join("out/data.html").display().to_string()
        );
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][0]["contentType"],
            "text/html"
        );
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][0]["outputKind"],
            "document"
        );
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][0]["hasSourceMap"],
            true
        );
        assert!(
            report["reportAst"]["transformGraph"]["exports"][0]["outputSpanCount"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][1]["contentType"],
            "image/svg+xml"
        );
    }

    #[test]
    fn transform_graph_report_records_source_map_sidecar_refs() {
        let collection_source_map = serde_json::to_value(test_source_map(8)).unwrap();
        let collection_output_spans = serde_json::to_value(vec![test_output_span(8)]).unwrap();
        let artifacts = vec![
            eng::TransformGraphArtifact {
                export_id: "main".to_owned(),
                input: "html".to_owned(),
                destination: Some("out/page.html".to_owned()),
                identity: Some(eng::FormatIdentity {
                    content_type: Some("text/html".to_owned()),
                    ..eng::FormatIdentity::default()
                }),
                primary: serde_json::json!({
                    "kind": "document"
                }),
                source_map: Some(test_source_map(4)),
                output_spans: vec![test_output_span(4)],
            },
            eng::TransformGraphArtifact {
                export_id: "stdout".to_owned(),
                input: "html".to_owned(),
                destination: None,
                identity: None,
                primary: serde_json::json!({
                    "kind": "document"
                }),
                source_map: Some(test_source_map(0)),
                output_spans: Vec::new(),
            },
            eng::TransformGraphArtifact {
                export_id: "collection".to_owned(),
                input: "joined".to_owned(),
                destination: Some("out/collection.json".to_owned()),
                identity: Some(eng::FormatIdentity {
                    content_type: Some("application/json".to_owned()),
                    ..eng::FormatIdentity::default()
                }),
                primary: serde_json::json!({
                    "kind": "collection",
                    "items": [{
                        "input": "primary",
                        "artifactId": "html",
                        "sourceMap": collection_source_map,
                        "outputSpans": collection_output_spans,
                    }],
                }),
                source_map: None,
                output_spans: vec![test_output_span(8)],
            },
        ];

        let report = transform_graph_report_from_artifacts(&artifacts);
        let mut rendered_report = cem_ml::report::Report::deterministic(
            vec!["graph.cem".to_owned()],
            vec![],
            cem_ml::report::ReportOptionsSnapshot {
                fail_level: eng::FailLevel::Validate,
                schema: None,
                content_type: None,
                base_uri: None,
            },
        );
        rendered_report.report_ast.transform_graph = Some(report.clone());
        let markdown = render_report_markdown(&rendered_report);
        assert!(markdown.contains("[sourceMapRef: out/page.html.map]"));
        assert!(markdown.contains("- main <- html -> out/page.html"));
        assert!(markdown.contains("[collectionItems: 1]"));

        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["exports"][0]["input"], "html");
        assert_eq!(value["exports"][0]["sourceMapRef"], "out/page.html.map");
        assert_eq!(value["exports"][0]["hasSourceMap"], true);
        assert_eq!(value["exports"][0]["outputSpanCount"], 1);
        assert!(value["exports"][1]["sourceMapRef"].is_null());
        assert_eq!(value["exports"][1]["hasSourceMap"], true);
        assert_eq!(
            value["exports"][2]["sourceMapRef"],
            "out/collection.json.map"
        );
        assert_eq!(value["exports"][2]["hasSourceMap"], true);
        assert_eq!(
            value["exports"][2]["collectionItems"][0]["input"],
            "primary"
        );
        assert_eq!(
            value["exports"][2]["collectionItems"][0]["artifactId"],
            "html"
        );
        assert_eq!(
            value["exports"][2]["collectionItems"][0]["hasSourceMap"],
            true
        );
        assert_eq!(
            value["exports"][2]["collectionItems"][0]["outputSpanCount"],
            1
        );
    }

    #[test]
    fn transform_graph_artifact_writer_emits_source_map_sidecars() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-graph-source-map-sidecar");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let out = root.join("out/page.html");
        let artifacts = vec![eng::TransformGraphArtifact {
            export_id: "main".to_owned(),
            input: "html".to_owned(),
            destination: Some(out.display().to_string()),
            identity: Some(eng::FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..eng::FormatIdentity::default()
            }),
            primary: serde_json::json!({
                "kind": "document",
                "content": "<main></main>"
            }),
            source_map: Some(test_source_map(13)),
            output_spans: vec![test_output_span(13)],
        }];
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let mut streams = Streams {
            stdout: &mut stdout,
            stderr: &mut stderr,
            quiet: false,
        };

        write_transform_graph_artifacts(&eng::EngineContext::default(), &artifacts, &mut streams)
            .unwrap();

        assert!(out.exists());
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "<main></main>");
        let source_map: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(format!("{}.map", out.display())).unwrap(),
        )
        .unwrap();
        assert!(source_map["frames"].is_array());
        assert_eq!(source_map["exportId"], "main");
        assert_eq!(source_map["input"], "html");
        assert_eq!(source_map["destination"], out.display().to_string());
        assert!(stdout.into_inner().is_empty());
    }

    #[test]
    fn transform_graph_artifact_writer_emits_collection_source_map_sidecars() {
        let root =
            std::env::temp_dir().join("cem-ml-cli-tests/transform-graph-collection-map-sidecar");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let out = root.join("out/collection.json");
        let source_map = serde_json::to_value(test_source_map(21)).unwrap();
        let output_spans = serde_json::to_value(vec![test_output_span(21)]).unwrap();
        let artifacts = vec![eng::TransformGraphArtifact {
            export_id: "joined".to_owned(),
            input: "collection".to_owned(),
            destination: Some(out.display().to_string()),
            identity: Some(eng::FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..eng::FormatIdentity::default()
            }),
            primary: serde_json::json!({
                "kind": "collection",
                "items": [{
                    "input": "primary",
                    "artifactId": "html",
                    "sourceMap": source_map,
                    "outputSpans": output_spans,
                }],
            }),
            source_map: None,
            output_spans: vec![test_output_span(21)],
        }];
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let mut streams = Streams {
            stdout: &mut stdout,
            stderr: &mut stderr,
            quiet: false,
        };

        write_transform_graph_artifacts(&eng::EngineContext::default(), &artifacts, &mut streams)
            .unwrap();

        assert!(out.exists());
        let collection: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(collection["kind"], "collection");
        assert_eq!(collection["items"][0]["artifactId"], "html");
        assert!(collection["items"][0]["sourceMap"].is_null());
        assert!(collection["items"][0]["outputSpans"].is_null());
        let sidecar: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(format!("{}.map", out.display())).unwrap(),
        )
        .unwrap();
        assert_eq!(sidecar["kind"], "collection");
        assert_eq!(sidecar["exportId"], "joined");
        assert_eq!(sidecar["input"], "collection");
        assert_eq!(sidecar["destination"], out.display().to_string());
        assert_eq!(sidecar["items"][0]["artifactId"], "html");
        assert!(sidecar["items"][0]["sourceMap"]["frames"].is_array());
        assert!(stdout.into_inner().is_empty());
    }

    #[test]
    fn transform_config_markdown_report_lists_graph_exports() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-report-md");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("data.cem"), "{p}").unwrap();
        std::fs::write(root.join("view.cem"), "{section | OK}").unwrap();
        let config = root.join("graph.cem");
        let report = root.join("report.md");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="data.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="view.cem" @template-content-type="text/cem-ml" |
      {export @id=main @out="out/{stem}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                "--config",
                config.to_str().unwrap(),
                "--config-schema",
                transform_config::TRANSFORM_CONFIG_SCHEMA_URI,
                "--report-md",
                report.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let markdown = std::fs::read_to_string(report).unwrap();
        assert!(markdown.contains("## transform graph"));
        assert!(markdown.contains("- exports: 1"));
        assert!(markdown.contains("main <- html ->"));
        assert!(markdown.contains("out/data.html"));
        assert!(markdown.contains("(text/html)"));
        assert!(markdown.contains("[sourceMap: yes, outputSpans: "));
        assert!(markdown.contains("[sourceMapRef: "));
    }

    #[test]
    fn transform_config_unknown_schema_fails_before_document_parsing() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-unknown-schema");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let config = root.join("graph.cem");
        std::fs::write(&config, "{run}").unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                "--config",
                config.to_str().unwrap(),
                "--config-schema",
                "https://cem.dev/ns/core/1",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stdout.is_empty(), "{stdout}");
        assert_stderr_contains_all(
            &stderr,
            &[
                "cem.transform_config.unsupported_schema_identity",
                "https://cem.dev/ns/cli/transform-config/1",
            ],
        );
    }

    #[test]
    fn transform_config_missing_output_binding_is_usage_error() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-missing-binding");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("data.cem"), "{p}").unwrap();
        std::fs::write(root.join("view.cem"), "{section | OK}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="data.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="view.cem" @template-content-type="text/cem-ml" |
      {export @id=main @out="out/{chapter}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stdout.is_empty(), "{stdout}");
        assert_stderr_contains_all(
            &stderr,
            &[
                "invalid run config",
                "output_binding_unknown",
                "unknown binding `chapter`",
            ],
        );
    }

    #[test]
    fn transform_config_duplicate_resolved_outputs_fail_before_write() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-duplicate-output");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("data.cem"), "{p}").unwrap();
        std::fs::write(root.join("page.cem"), "{article | OK}").unwrap();
        std::fs::write(root.join("alt.cem"), "{section | OK}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="data.cem" @content-type="text/cem-ml" |
    {transform @id=page @src="page.cem" @template-content-type="text/cem-ml" |
      {export @id=main @out="out/{stem}.html" @content-type="text/html"}
    }
    {transform @id=alt @src="alt.cem" @template-content-type="text/cem-ml" |
      {export @id=alt-out @out="out/{stem}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["transform", "--config", config.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE);
        assert!(stdout.is_empty(), "{stdout}");
        assert_stderr_contains_all(&stderr, &["duplicate_destination", "out/data.html"]);
        assert!(!root.join("out/data.html").exists());
    }

    #[test]
    fn transform_config_request_helper_resolves_relative_paths_from_config_dir() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-helper");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("data.cem"), "{p}").unwrap();
        std::fs::write(root.join("view.cem"), "{section | OK}").unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="data.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="view.cem" @template-content-type="text/cem-ml" |
      {export @id=main @out="out/book.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();
        let parsed = parse_cli(&["transform", "--config", config.to_str().unwrap()]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };
        let context = context(&cli::ContextOptions::default());

        let (request, config_source_uri) = match transform_graph_request_from_args(&context, &args)
        {
            Ok(request) => request,
            Err(_) => panic!("transform config should lower to graph request"),
        };

        assert_eq!(config_source_uri, config.display().to_string());
        assert_eq!(
            request.imports[0].input.uri,
            root.join("data.cem").display().to_string()
        );
        assert_eq!(
            request.stages[0].template.uri,
            root.join("view.cem").display().to_string()
        );
        assert_eq!(
            request.exports[0].destination.as_deref(),
            Some(root.join("out/book.html").display().to_string().as_str())
        );
        assert_eq!(request.stages[0].primary_input, "book");
        assert_eq!(request.exports[0].input, "html");
    }

    #[test]
    fn transform_config_request_helper_lowers_entrypoint_and_expanded_params() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-entrypoint");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("chapter-one.cem"), "{p | Chapter}").unwrap();
        std::fs::write(
            root.join("page.cem"),
            r#"{module |
  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {article | {$ title }}}
  }
}"#,
        )
        .unwrap();
        let config = root.join("graph.cem");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="chapter-one.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="page.cem" @template-content-type="text/cem-ml" @template-schema="https://cem.dev/ns/template/cem-native/1" @entrypoint="card" |
      {param @name="title" @value="{stem}"}
      {export @id=main @out="out/{stem}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();
        let parsed = parse_cli(&["transform", "--config", config.to_str().unwrap()]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };
        let context = context(&cli::ContextOptions::default());

        let (request, _) = match transform_graph_request_from_args(&context, &args) {
            Ok(request) => request,
            Err(_) => panic!("transform config should lower entrypoint and params"),
        };

        assert_eq!(
            request.stages[0].template_entrypoint.name.as_deref(),
            Some("card")
        );
        assert_eq!(
            request.stages[0].params.get("title"),
            Some(&serde_json::json!("chapter-one"))
        );
        assert_eq!(
            request.stages[0].execution_policy.runtime_phase,
            eng::TransformRuntimePhase::CemNativeModules
        );
    }

    #[test]
    fn transform_config_executes_relative_imported_cem_native_module_with_sidecar() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-config-imported-module");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("chapter-one.cem"), "{p | Chapter}").unwrap();
        std::fs::write(
            root.join("page.cem"),
            r#"{@doc cem-ml 1}
{module |
  {import @as="ui" @src="ui.cem" @content-type="text/cem-ml"}
  {template @name="page" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {main | {call @from="ui" @template="card" @with:title="{title}"}}}
  }
}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("ui.cem"),
            r#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {article | {$ title }}}
  }
}"#,
        )
        .unwrap();
        let config = root.join("graph.cem");
        let report = root.join("report.json");
        std::fs::write(
            &config,
            r#"{run |
  {import @id=book @src="chapter-one.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="page.cem" @template-content-type="text/cem-ml" @template-schema="https://cem.dev/ns/template/cem-native/1" @entrypoint="page" |
      {param @name="title" @value="{stem}"}
      {export @id=main @out="out/{stem}.html" @content-type="text/html"}
    }
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                "--config",
                config.to_str().unwrap(),
                "--report-json",
                report.to_str().unwrap(),
            ],
        );

        let out = root.join("out/chapter-one.html");
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "<main><article>chapter-one</article></main>"
        );
        let sidecar = format!("{}.map", out.display());
        let sidecar_json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&sidecar).unwrap()).unwrap();
        assert_eq!(sidecar_json["exportId"], "main");
        assert_eq!(sidecar_json["input"], "html");
        assert_eq!(sidecar_json["destination"], out.display().to_string());
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report).unwrap()).unwrap();
        assert_eq!(report["summary"]["hardViolationCount"], 0);
        assert_eq!(
            report["reportAst"]["transformGraph"]["exports"][0]["sourceMapRef"],
            sidecar
        );
    }

    #[test]
    fn transform_warnings_go_to_stderr_without_report_destination() {
        let data = write_fixture("transform-run-warning-data.cem", "{p @label=\"source\"}");
        let template = write_fixture("transform-run-warning-template.cem", "{section | Done}");

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--to-content-type",
                "text/html",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert_eq!(stdout, "<section>Done</section>");
        assert_stderr_contains_all(&stderr, &["warning", "unknown_html_attribute"]);
    }

    #[test]
    fn transform_report_destination_suppresses_warning_stderr() {
        let data = write_fixture(
            "transform-run-report-warning-data.cem",
            "{p @label=\"source\"}",
        );
        let template = write_fixture(
            "transform-run-report-warning-template.cem",
            "{section | Done}",
        );
        let report_path = std::env::temp_dir().join("cem-ml-cli-tests/transform-report.json");
        let _ = std::fs::remove_file(&report_path);

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--to-content-type",
                "text/html",
                "--report-json",
                report_path.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert_eq!(stdout, "<section>Done</section>");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report_path).unwrap()).unwrap();
        assert_eq!(report["summary"]["warningCount"], 1);
    }

    #[test]
    fn transform_direct_cli_renders_relative_imported_cem_native_module() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-import-direct");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let data = root.join("data.cem");
        let template = root.join("page.cem");
        let imported = root.join("ui.cem");
        let report = root.join("report.json");
        std::fs::write(&data, "{p | Source}").unwrap();
        std::fs::write(
            &template,
            r#"{@doc cem-ml 1}
{module |
  {import @as="ui" @src="ui.cem" @content-type="text/cem-ml"}
  {template @name="page" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {main | {call @from="ui" @template="card" @with:title="{title}"}}}
  }
}"#,
        )
        .unwrap();
        std::fs::write(
            &imported,
            r#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {article | {$ title }}}
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--template-schema",
                "https://cem.dev/ns/template/cem-native/1",
                "--template-entrypoint",
                "page",
                "--param",
                "title=Imported",
                "--to-content-type",
                "text/html",
                "--report-json",
                report.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert_eq!(stdout, "<main><article>Imported</article></main>");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report).unwrap()).unwrap();
        assert_eq!(report["summary"]["hardViolationCount"], 0);
        assert_eq!(report["reportAst"]["transform"]["hasSourceMap"], true);
    }

    #[test]
    fn transform_direct_cli_reports_imported_call_diagnostics() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-import-diagnostic");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let data = root.join("data.cem");
        let template = root.join("page.cem");
        let imported = root.join("ui.cem");
        let report = root.join("report.json");
        std::fs::write(&data, "{p | Source}").unwrap();
        std::fs::write(
            &template,
            r#"{@doc cem-ml 1}
{module |
  {import @as="ui" @src="ui.cem" @content-type="text/cem-ml"}
  {body | {main | {call @from="ui" @template="card"}}}
}"#,
        )
        .unwrap();
        std::fs::write(
            &imported,
            r#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {article | {$ title }}}
  }
}"#,
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--template-schema",
                "https://cem.dev/ns/template/cem-native/1",
                "--to-content-type",
                "text/html",
                "--report-json",
                report.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report).unwrap()).unwrap();
        assert!(report["summary"]["hardViolationCount"].as_u64().unwrap() > 0);
        let diagnostics = report["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == "cem.transform_template.param_required"
                && diagnostic["uri"] == template.display().to_string()
        }));
    }

    #[test]
    fn transform_report_records_source_map_sidecar_ref() {
        let data = write_fixture("transform-run-report-map-data.cem", "{p @id=\"source\"}");
        let template = write_fixture("transform-run-report-map-template.cem", "{section | Done}");
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-report-map");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let out = root.join("out.html");
        let report_json = root.join("report.json");
        let report_md = root.join("report.md");

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--to-content-type",
                "text/html",
                "--out",
                out.to_str().unwrap(),
                "--report-json",
                report_json.to_str().unwrap(),
                "--report-md",
                report_md.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty(), "{stdout}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report_json).unwrap()).unwrap();
        assert_eq!(
            report["reportAst"]["transform"]["input"],
            data.display().to_string()
        );
        assert_eq!(
            report["reportAst"]["transform"]["destination"],
            out.display().to_string()
        );
        assert_eq!(report["reportAst"]["transform"]["hasSourceMap"], true);
        assert!(
            report["reportAst"]["transform"]["outputSpanCount"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(
            report["reportAst"]["transform"]["sourceMapRef"],
            format!("{}.map", out.display())
        );
        let markdown = std::fs::read_to_string(report_md).unwrap();
        assert!(markdown.contains("## transform"));
        assert!(markdown.contains("[sourceMap: yes, outputSpans: "));
        assert!(markdown.contains(&format!("[sourceMapRef: {}.map]", out.display())));
    }

    #[test]
    fn transform_report_stdout_omits_source_map_ref() {
        let data = write_fixture("transform-run-report-stdout-map-data.cem", "{p | source}");
        let template = write_fixture(
            "transform-run-report-stdout-map-template.cem",
            "{section | Done}",
        );
        let root = std::env::temp_dir().join("cem-ml-cli-tests/transform-report-stdout-map");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let report_json = root.join("report.json");
        let report_md = root.join("report.md");

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "transform",
                data.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--to-content-type",
                "text/html",
                "--report-json",
                report_json.to_str().unwrap(),
                "--report-md",
                report_md.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK);
        assert_eq!(stdout, "<section>Done</section>");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report_json).unwrap()).unwrap();
        assert_eq!(
            report["reportAst"]["transform"]["input"],
            data.display().to_string()
        );
        assert!(report["reportAst"]["transform"]["destination"].is_null());
        assert_eq!(report["reportAst"]["transform"]["hasSourceMap"], true);
        assert!(
            report["reportAst"]["transform"]["outputSpanCount"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(report["reportAst"]["transform"]["sourceMapRef"].is_null());
        let markdown = std::fs::read_to_string(report_md).unwrap();
        assert!(markdown.contains("- primary <- "));
        assert!(markdown.contains(" -> <stdout> [document]"));
        assert!(markdown.contains("[sourceMap: yes, outputSpans: "));
        assert!(!markdown.contains("[sourceMapRef: "));
    }

    #[test]
    fn transform_request_helper_reads_data_template_and_sets_identities() {
        let data = write_fixture("transform-helper-data.xml", "<items/>");
        let template = write_fixture(
            "transform-helper-view.xsl",
            r#"<xsl:stylesheet version="1.0"/>"#,
        );
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template.to_str().unwrap(),
            "--template-content-type",
            "application/xslt+xml",
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let request = match transform_request_from_args(&eng::EngineContext::default(), &args) {
            Ok(request) => request,
            Err(_) => panic!("transform request helper should succeed"),
        };

        assert_eq!(request.data.bytes, b"<items/>");
        assert_eq!(
            request.template.bytes,
            br#"<xsl:stylesheet version="1.0"/>"#
        );
        assert_eq!(
            request
                .data
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("application/xml")
        );
        assert_eq!(
            request
                .template
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("application/xslt+xml")
        );
        assert_eq!(request.template_kind, eng::TransformTemplateKind::Xslt);
        assert_eq!(
            request
                .target
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/html")
        );
    }

    #[test]
    fn transform_request_helper_reads_custom_template_with_registered_resolver() {
        let data = write_fixture("transform-helper-custom-template-data.xml", "<items/>");
        let template_uri = "cem+vfs://workspace/view.xsl";
        let context = context_with_read_resolver(
            ResolvePurpose::Template,
            StaticReadResolver {
                uri: template_uri,
                bytes: br#"<xsl:stylesheet version="1.0"/>"#,
                content_type: Some("application/xslt+xml"),
            },
        );
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template_uri,
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let request = match transform_request_from_args(&context, &args) {
            Ok(request) => request,
            Err(_) => panic!("transform request helper should read registered template"),
        };

        assert_eq!(request.template.uri, template_uri);
        assert_eq!(
            request.template.bytes,
            br#"<xsl:stylesheet version="1.0"/>"#
        );
        assert_eq!(request.template_kind, eng::TransformTemplateKind::Xslt);
        assert_eq!(
            request.template.root_scope.default_content_type.as_deref(),
            Some("application/xslt+xml")
        );
    }

    #[test]
    fn transform_request_helper_accepts_cem_native_template_identity() {
        let data = write_fixture("transform-helper-cem-template-data.xml", "<items/>");
        let template = write_fixture("transform-helper-view.cem", "{template | {p Hello}}");
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template.to_str().unwrap(),
            "--template-content-type",
            "text/cem-ml",
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let request = match transform_request_from_args(&eng::EngineContext::default(), &args) {
            Ok(request) => request,
            Err(_) => panic!("transform request helper should accept CEM-native templates"),
        };

        assert_eq!(request.template_kind, eng::TransformTemplateKind::CemNative);
        assert_eq!(
            request.template.root_scope.default_content_type.as_deref(),
            Some("text/cem-ml")
        );
    }

    #[test]
    fn transform_request_helper_infers_cemt_template_schema_and_runtime_adapter() {
        let data = write_fixture("transform-helper-cemt-data.xml", "<items/>");
        let template = write_fixture(
            "transform-helper-view.cemt",
            r#"{transform @to-content-type="text/html" | {template | {p Hello}}}"#,
        );
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template.to_str().unwrap(),
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };
        let context = context(&cli::ContextOptions::default());

        let request = match transform_request_from_args(&context, &args) {
            Ok(request) => request,
            Err(_) => panic!("transform request helper should accept CEMT templates"),
        };

        assert_eq!(request.template_kind, eng::TransformTemplateKind::CemNative);
        assert_eq!(
            request.template.root_scope.default_content_type.as_deref(),
            Some(cem_ml::schema::registry::CEM_TRANSFORM_CONTENT_TYPE)
        );
        assert_eq!(
            request.template.root_scope.schema.as_deref(),
            Some(cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI)
        );
        let template_identity = request.template.identity.as_ref().unwrap();
        assert_eq!(
            template_identity.content_type.as_deref(),
            Some(cem_ml::schema::registry::CEM_TRANSFORM_CONTENT_TYPE)
        );
        assert_eq!(
            template_identity.schema.as_deref(),
            Some(cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI)
        );
        let adapter = match request
            .context
            .template_adapter_registry
            .select_adapter(template_identity)
        {
            cem_ml::transform_template::TransformTemplateAdapterLookup::Matched(adapter) => adapter,
            other => panic!("expected executable CEM-QL adapter, got {other:?}"),
        };
        assert_eq!(
            adapter.id(),
            cem_ml_transform_cem_ql::CEM_QL_TEMPLATE_ADAPTER_ID
        );
    }

    #[test]
    fn transform_request_helper_lowers_template_entrypoint_and_params() {
        let data = write_fixture("transform-helper-entrypoint-data.cem", "{p Hi}");
        let template = write_fixture(
            "transform-helper-entrypoint-view.cem",
            r#"{module |
  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {article | {$ title }}}
  }
}"#,
        );
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "text/cem-ml",
            "--template",
            template.to_str().unwrap(),
            "--template-content-type",
            "text/cem-ml",
            "--template-schema",
            "https://cem.dev/ns/template/cem-native/1",
            "--template-entrypoint",
            "card",
            "--param",
            "title=Intro",
            "--to-content-type",
            "text/html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let request = match transform_request_from_args(&eng::EngineContext::default(), &args) {
            Ok(request) => request,
            Err(_) => panic!("transform request helper should lower entrypoint and params"),
        };

        assert_eq!(request.template_kind, eng::TransformTemplateKind::CemNative);
        assert_eq!(request.template_entrypoint.name.as_deref(), Some("card"));
        assert_eq!(
            request.params.get("title"),
            Some(&serde_json::json!("Intro"))
        );
        assert_eq!(
            request.execution_policy.runtime_phase,
            eng::TransformRuntimePhase::CemNativeModules
        );
    }

    #[test]
    fn transform_request_helper_rejects_bad_cli_params() {
        let data = write_fixture("transform-helper-bad-param-data.cem", "{p Hi}");
        let template = write_fixture("transform-helper-bad-param-view.cem", "{p | Hi}");
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "text/cem-ml",
            "--template",
            template.to_str().unwrap(),
            "--template-content-type",
            "text/cem-ml",
            "--param",
            "missing-equals",
            "--to-content-type",
            "text/html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let err = transform_request_from_args(&eng::EngineContext::default(), &args)
            .err()
            .expect("bad param should fail");
        let CliRequestError::Usage(message) = err else {
            panic!("expected usage error");
        };
        assert!(message.contains("NAME=VALUE"));
    }

    #[test]
    fn cli_context_registers_executable_cem_ql_transform_adapter() {
        let data = write_fixture("transform-helper-cli-adapter-data.cem", "{p Hi}");
        let template = write_fixture("transform-helper-cli-adapter-view.cem", "{p | Hello}");
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "text/cem-ml",
            "--template",
            template.to_str().unwrap(),
            "--template-content-type",
            "text/cem-ml",
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };
        let context = context(&cli::ContextOptions::default());

        let request = match transform_request_from_args(&context, &args) {
            Ok(request) => request,
            Err(_) => panic!("transform request helper should accept CEM-native templates"),
        };
        let template_identity = request.template.identity.as_ref().unwrap();
        let adapter = match request
            .context
            .template_adapter_registry
            .select_adapter(template_identity)
        {
            cem_ml::transform_template::TransformTemplateAdapterLookup::Matched(adapter) => adapter,
            other => panic!("expected executable CEM-QL adapter, got {other:?}"),
        };

        assert_eq!(
            adapter.id(),
            cem_ml_transform_cem_ql::CEM_QL_TEMPLATE_ADAPTER_ID
        );
    }

    #[test]
    fn transform_request_helper_uses_runtime_template_adapter_registry() {
        let data = write_fixture(
            "transform-helper-custom-template-schema-data.xml",
            "<items/>",
        );
        let template = write_fixture("transform-helper-view-v2.cem", "{template | {p Hello}}");
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template.to_str().unwrap(),
            "--template-schema",
            "https://cem.dev/ns/template/cem-native/2",
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };
        let mut context = eng::EngineContext::default();
        context.template_adapter_registry.register(
            cem_ml::transform_template::StaticTransformTemplateAdapter::new(
                "cem-native-template-v2",
                eng::TransformTemplateKind::CemNative,
                &[],
                &["https://cem.dev/ns/template/cem-native/2"],
                &[],
            ),
        );

        let request = match transform_request_from_args(&context, &args) {
            Ok(request) => request,
            Err(_) => panic!("transform request helper should use runtime template adapter"),
        };

        assert_eq!(request.template_kind, eng::TransformTemplateKind::CemNative);
        assert_eq!(
            request.template.root_scope.schema.as_deref(),
            Some("https://cem.dev/ns/template/cem-native/2")
        );
    }

    #[test]
    fn transform_request_helper_does_not_use_input_resolver_for_template() {
        let data = write_fixture(
            "transform-helper-input-purpose-template-data.xml",
            "<items/>",
        );
        let template_uri = "cem+vfs://workspace/view.xsl";
        let context = context_with_read_resolver(
            ResolvePurpose::Input,
            StaticReadResolver {
                uri: template_uri,
                bytes: br#"<xsl:stylesheet version="1.0"/>"#,
                content_type: Some("application/xslt+xml"),
            },
        );
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template_uri,
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let err = transform_request_from_args(&context, &args)
            .err()
            .expect("input resolver must not satisfy template reads");

        let CliRequestError::Engine(EngineError::Io { path, source }) = err else {
            panic!("expected engine I/O error for custom template");
        };
        assert_eq!(path, PathBuf::from(template_uri));
        let message = source.to_string();
        assert!(message.contains("remote/custom URI resolvers are not implemented"));
        assert!(message.contains(template_uri));
    }

    #[test]
    fn transform_request_helper_rejects_unknown_template_identity() {
        let data = write_fixture("transform-helper-unknown-template-data.xml", "<items/>");
        let template = write_fixture("transform-helper-view.bin", "template");
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template.to_str().unwrap(),
            "--template-content-type",
            "application/octet-stream",
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let err = transform_request_from_args(&eng::EngineContext::default(), &args)
            .err()
            .expect("unknown template identity should fail");
        let CliRequestError::Usage(message) = err else {
            panic!("expected usage error");
        };
        assert!(message.contains(eng::TRANSFORM_TEMPLATE_UNSUPPORTED_CODE));
        assert!(message.contains("application/octet-stream"));
    }

    #[test]
    fn transform_request_helper_rejects_remote_template_without_resolver() {
        let data = write_fixture("transform-helper-remote-template-data.xml", "<items/>");
        let template_uri = "https://example.test/view.xsl";
        let parsed = parse_cli(&[
            "transform",
            data.to_str().unwrap(),
            "--data-content-type",
            "application/xml",
            "--template",
            template_uri,
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ]);
        let cli::Command::Transform(args) = parsed.command else {
            panic!("expected transform command");
        };

        let err = transform_request_from_args(&eng::EngineContext::default(), &args)
            .err()
            .expect("remote template should be rejected without resolver");

        let CliRequestError::Engine(EngineError::Io { path, source }) = err else {
            panic!("expected engine I/O error for remote template");
        };
        assert_eq!(path, PathBuf::from(template_uri));
        let message = source.to_string();
        assert!(message.contains("remote/custom URI resolvers are not implemented"));
        assert!(message.contains(template_uri));
    }

    #[test]
    fn reserved_schema_sample_exits_two() {
        let (outcome, _, _) = run(&NotImplementedEngine, &["schema", "sample"]);
        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
    }

    #[test]
    fn parse_with_not_implemented_engine_exits_zero_and_warns() {
        let p = write_fixture("parse-not-impl.cem", "{x}");
        let (outcome, _, stderr) = run(&NotImplementedEngine, &["parse", p.to_str().unwrap()]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stderr.contains("parser engine not yet implemented"));
    }

    #[test]
    fn parse_missing_file_exits_six() {
        let (outcome, _, stderr) = run(
            &NotImplementedEngine,
            &["parse", "/nonexistent/path-cem-ml-test.cem"],
        );
        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stderr.contains("I/O error"));
    }

    #[test]
    fn parse_remote_uri_input_is_rejected_without_resolver() {
        let uri = "https://example.test/input.cem";
        assert_remote_input_uri_rejected(&["parse", uri], uri);
    }

    #[test]
    fn parse_non_local_file_uri_input_is_rejected() {
        let uri = "file://example.test/input.cem";
        assert_non_local_file_uri_input_rejected(&["parse", uri], uri);
    }

    #[test]
    fn non_local_file_uri_inputs_are_rejected_across_cli_commands() {
        const URI: &str = "file://example.test/input.cem";
        let cases: Vec<(&str, Vec<&str>)> = vec![
            ("validate", vec!["validate", "--format", "json", URI]),
            (
                "validate-input-spec",
                vec![
                    "validate",
                    "--input-spec",
                    "uri=file://example.test/input.cem",
                ],
            ),
            ("check", vec!["check", "--format", "json", URI]),
            ("convert", vec!["convert", URI]),
            ("inspect", vec!["inspect", URI]),
            ("trace", vec!["trace", URI]),
            ("bench", vec!["bench", "--format", "json", URI]),
        ];

        for (_name, args) in cases {
            assert_non_local_file_uri_input_rejected(&args, URI);
        }
    }

    #[test]
    fn custom_uri_inputs_are_rejected_across_cli_commands() {
        const URI: &str = "cem+vfs://workspace/input.cem";
        let cases: Vec<Vec<&str>> = vec![
            vec!["validate", "--format", "json", URI],
            vec![
                "validate",
                "--input-spec",
                "uri=cem+vfs://workspace/input.cem",
            ],
            vec!["check", "--format", "json", URI],
            vec!["convert", URI],
            vec!["inspect", URI],
            vec!["trace", URI],
            vec!["bench", "--format", "json", URI],
        ];

        for args in cases {
            assert_remote_input_uri_rejected(&args, URI);
        }
    }

    #[test]
    fn parse_with_fake_engine_emits_json_to_stdout() {
        let p = write_fixture("parse-fake.cem", "{x}");
        let (outcome, stdout, _) = run(&FakeEngine, &["parse", p.to_str().unwrap()]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["kind"], "fake-parse");
        assert_eq!(v["projection"], "dom-json");
    }

    #[test]
    fn parse_file_uri_input_reads_local_input() {
        let p = write_fixture("parse-file-uri-input.cem", "{x}");
        let file_uri = local_file_uri(&p);
        let (outcome, stdout, stderr) = run(&FakeEngine, &["parse", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["input"], file_uri);
    }

    #[test]
    fn parse_localhost_file_uri_input_reads_local_input() {
        let p = write_fixture("parse-localhost-file-uri-input.cem", "{x}");
        let file_uri = localhost_file_uri(&p);
        let (outcome, stdout, stderr) = run(&FakeEngine, &["parse", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["input"], file_uri);
    }

    #[test]
    fn parse_percent_escaped_file_uri_input_reads_local_input() {
        let p = write_fixture("parse-percent escaped-input.cem", "{x}");
        let uri = local_file_uri(&p).replace(' ', "%20");
        let (outcome, stdout, stderr) = run(&FakeEngine, &["parse", &uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["input"], uri);
    }

    #[test]
    fn parse_malformed_percent_escaped_file_uri_input_is_rejected() {
        let uri = "file:///tmp/cem%zz/input.cem";
        assert_non_local_file_uri_input_rejected(&["parse", uri], uri);
    }

    #[test]
    fn parse_writes_to_out_path_and_keeps_stdout_empty() {
        let p = write_fixture("parse-out.cem", "{x}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/parse-out.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, _) = run(
            &FakeEngine,
            &[
                "parse",
                "--out",
                out_path.to_str().unwrap(),
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(
            stdout.is_empty(),
            "stdout should be empty when --out is used"
        );
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-parse");
    }

    #[test]
    fn parse_file_uri_out_destination_writes_local_output() {
        let p = write_fixture("parse-file-uri-out.cem", "{x}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/parse-file-uri-out.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "parse",
                "--out",
                &format!("file://{}", out_path.display()),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-parse");
    }

    #[test]
    fn parse_localhost_file_uri_out_destination_writes_local_output() {
        let p = write_fixture("parse-localhost-file-uri-out.cem", "{x}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/parse-localhost-file-uri-out.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "parse",
                "--out",
                &localhost_file_uri(&out_path),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-parse");
    }

    #[test]
    fn parse_percent_escaped_file_uri_out_destination_writes_local_output() {
        let p = write_fixture("parse-percent-escaped-file-uri-out.cem", "{x}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/parse-percent escaped-file-uri-out.json");
        let _ = std::fs::remove_file(&out_path);
        let out_uri = local_file_uri(&out_path).replace(' ', "%20");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["parse", "--out", &out_uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-parse");
    }

    #[test]
    fn parse_malformed_percent_escaped_file_uri_out_destination_is_rejected() {
        let p = write_fixture("parse-malformed-percent-escaped-file-uri-out.cem", "{x}");
        let uri = "file:///tmp/cem%zz/out.json";
        assert_non_local_file_uri_output_rejected(
            &["parse", "--out", uri, p.to_str().unwrap()],
            uri,
        );
    }

    #[test]
    fn parse_writes_side_report_files_when_requested() {
        let p = write_fixture("parse-report.cem", "{x}");
        let report_dir = std::env::temp_dir().join("cem-ml-cli-tests/parse-report-dir");
        let _ = std::fs::remove_dir_all(&report_dir);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "parse",
                "--report-json",
                report_dir.to_str().unwrap(),
                "--report-md",
                report_dir.to_str().unwrap(),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let primary: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(primary["kind"], "fake-parse");
        let report_path = report_dir.join("cem-ml.report.json");
        let markdown_path = report_dir.join("cem-ml.report.md");
        assert!(report_path.is_file());
        assert!(markdown_path.is_file());
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report_path).unwrap()).unwrap();
        assert_eq!(report["summary"]["inputCount"], 1);
        assert_eq!(report["diagnostics"][0]["code"], "fake.engine.placeholder");
        let markdown = std::fs::read_to_string(markdown_path).unwrap();
        assert!(markdown.contains("# cem-ml report"));
        assert!(markdown.contains("- info: 1"));
    }

    #[test]
    fn parse_remote_uri_out_destination_is_rejected_without_resolver() {
        let p = write_fixture("parse-remote-out.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "parse",
                "--out",
                "https://example.test/out.json",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/out.json");
    }

    #[test]
    fn non_local_file_uri_out_destinations_are_rejected_across_cli_commands() {
        let input = write_fixture("non-local-file-uri-out.cem", "{x}");
        let input = input.to_str().unwrap();
        const URI: &str = "file://example.test/out.json";
        let cases: Vec<Vec<&str>> = vec![
            vec!["parse", "--out", URI, input],
            vec!["convert", "--out", URI, input],
            vec!["inspect", "--out", URI, input],
            vec!["trace", "--out", URI, input],
        ];

        for args in cases {
            assert_non_local_file_uri_output_rejected(&args, URI);
        }
    }

    #[test]
    fn custom_uri_out_destinations_are_rejected_across_cli_commands() {
        let input = write_fixture("custom-uri-out.cem", "{x}");
        let input = input.to_str().unwrap();
        const URI: &str = "cem+vfs://workspace/out.json";
        let cases: Vec<Vec<&str>> = vec![
            vec!["parse", "--out", URI, input],
            vec!["convert", "--out", URI, input],
            vec!["inspect", "--out", URI, input],
            vec!["trace", "--out", URI, input],
        ];

        for args in cases {
            assert_remote_output_uri_rejected(&args, URI);
        }
    }

    #[test]
    fn parse_remote_uri_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("parse-remote-report.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "parse",
                "--report-json",
                "https://example.test/parse-report.json",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-parse\""));
        assert_stderr_contains_all(&stderr, &["parse report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/parse-report.json");
    }

    #[test]
    fn parse_remote_uri_markdown_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("parse-remote-md-report.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "parse",
                "--report-md",
                "https://example.test/parse-report.md",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-parse\""));
        assert_stderr_contains_all(&stderr, &["parse report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/parse-report.md");
    }

    #[test]
    fn parse_custom_uri_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("parse-custom-report.cem", "{x}");
        let uri = "cem+vfs://workspace/parse-report.json";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["parse", "--report-json", uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-parse\""));
        assert_stderr_contains_all(&stderr, &["parse report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn parse_custom_uri_markdown_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("parse-custom-md-report.cem", "{x}");
        let uri = "cem+vfs://workspace/parse-report.md";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["parse", "--report-md", uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-parse\""));
        assert_stderr_contains_all(&stderr, &["parse report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn parse_non_local_file_uri_markdown_report_destination_is_rejected() {
        let p = write_fixture("parse-non-local-file-uri-md-report.cem", "{x}");
        let uri = "file://example.test/parse-report.md";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["parse", "--report-md", uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-parse\""));
        assert_stderr_contains_all(&stderr, &["parse report write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn validate_emits_report_with_contract_field_names() {
        let p = write_fixture("validate.cem", "{x}");
        let (outcome, stdout, _) = run(&FakeEngine, &["validate", p.to_str().unwrap()]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["generatedAt"], "1970-01-01T00:00:00.000Z");
        assert!(v["inputs"].is_array());
        for k in [
            "inputCount",
            "infoCount",
            "warningCount",
            "errorCount",
            "fatalCount",
            "hardViolationCount",
        ] {
            assert!(v["summary"][k].is_number(), "missing summary.{k}");
        }
        for k in ["failLevel", "schema", "contentType", "baseUri"] {
            assert!(v["options"].get(k).is_some(), "missing options.{k}");
        }
        assert_eq!(v["options"]["failLevel"], "validate");
    }

    #[test]
    fn validate_records_context_in_options() {
        let p = write_fixture("validate-ctx.cem", "{x}");
        let (_, stdout, _) = run(
            &FakeEngine,
            &[
                "validate",
                "--schema",
                "schema-uri",
                "--content-type",
                "application/cem",
                "--base-uri",
                "file:///x/",
                p.to_str().unwrap(),
            ],
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["options"]["schema"], "schema-uri");
        assert_eq!(v["options"]["contentType"], "application/cem");
        assert_eq!(v["options"]["baseUri"], "file:///x/");
    }

    #[test]
    fn validate_unknown_input_identity_reports_unsupported_adapter() {
        let p = write_fixture("validate-unknown-input-identity.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/unknown",
                "--schema",
                "https://example.test/ns/widgets/1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|diag| {
            diag["code"] == "cem.lifecycle.adapter_unsupported"
                && diag["message"].as_str().is_some_and(|message| {
                    message.contains("content type `application/unknown`")
                        && message.contains("schema `https://example.test/ns/widgets/1`")
                })
        }));
    }

    #[test]
    fn validate_unknown_input_schema_reports_unsupported_adapter() {
        let p = write_fixture("validate-unknown-input-schema.data", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--schema",
                "https://example.test/ns/widgets/1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|diag| {
            diag["code"] == "cem.lifecycle.adapter_unsupported"
                && diag["message"].as_str().is_some_and(|message| {
                    message.contains("schema `https://example.test/ns/widgets/1`")
                })
        }));
    }

    #[test]
    fn validate_transform_config_schema_selects_cem_input_adapter() {
        let p = write_fixture(
            "validate-transform-config-schema.cem",
            r#"{@doc cem-ml 1}
{run |
  {import @id=book @src="book.cem" @content-type="text/cem-ml" |
    {transform @id=html @src="page.cem" @template-content-type="text/cem-ml" |
      {export @id=main @out="out/book.html" @content-type="text/html"}
    }
  }
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--schema",
                cem_ml::transform_config::TRANSFORM_CONFIG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.transform_config."))));
    }

    #[test]
    fn validate_transform_config_schema_reports_graph_schema_diagnostics() {
        let p = write_fixture(
            "validate-transform-config-schema-invalid.cem",
            r#"{@doc cem-ml 1}
{run |
  {import @id=book}
  {transform @id=html}
  {export @id=main}
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--schema",
                cem_ml::transform_config::TRANSFORM_CONFIG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        for code in [
            "cem.transform_config.import_src_missing",
            "cem.transform_config.transform_src_missing",
            "cem.transform_config.export_out_missing",
        ] {
            assert!(
                diagnostics.iter().any(|diag| diag["code"] == code),
                "missing diagnostic {code}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn validate_native_template_schema_selects_cem_input_adapter() {
        let p = write_fixture(
            "validate-native-template-schema.cem",
            r#"@doc cem-ml 1
{module |
  {template @name="page" |
    {body | {p | Hi}}
  }
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--schema",
                cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn validate_cem_transform_schema_selects_cem_input_adapter() {
        let p = write_fixture(
            "validate-cem-transform-schema.cemt",
            r#"@doc cem-ml 1
{module |
  {template @name="main" |
    {body | {p | Hi}}
  }
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--schema",
                cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn validate_cem_ql_source_uses_cem_ql_parser() {
        let p = write_fixture(
            "validate-cem-ql-source.cemql",
            r#"module "https://example.test/queries/main"

declare variable count := 2

count + 1"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/cem-ql",
                "--schema",
                cem_ml::schema::registry::CEM_QL_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_cem_ql_source_reports_parser_diagnostics() {
        let p = write_fixture(
            "validate-cem-ql-source-invalid.cemql",
            r#"module "https://example.test/queries/broken"

declare variable broken := 1 +"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/vnd.cem.query+cem-ql",
                "--schema",
                cem_ml::schema::registry::CEM_QL_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.ql.parse_error"));
    }

    #[test]
    fn validate_json_source_uses_json_parser() {
        let p = write_fixture(
            "validate-json-source.json",
            r#"{"message":"Hello","items":[1,2,3],"enabled":true}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/json; charset=utf-8",
                "--schema",
                cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_json_source_reports_parser_diagnostics() {
        let p = write_fixture("validate-json-source-invalid.json", r#"{"broken": true,}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/json",
                "--schema",
                cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.json.parse_error"));
    }

    #[test]
    fn validate_yaml_source_uses_yaml_parser() {
        let p = write_fixture(
            "validate-yaml-source.yaml",
            r#"---
service:
  name: catalog
  enabled: true
  ports:
    - 8080
    - 8443
---
owner:
  name: CEM
  contacts:
    - ops@example.test
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/yaml; charset=utf-8",
                "--schema",
                cem_ml::schema::registry::YAML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_yaml_source_reports_parser_diagnostics() {
        let p = write_fixture(
            "validate-yaml-source-invalid.yaml",
            r#"items:
  - one
  - [unterminated
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/yaml",
                "--schema",
                cem_ml::schema::registry::YAML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.yaml.parse_error"));
    }

    #[test]
    fn validate_yaml_source_reports_unsafe_tag_diagnostics() {
        let p = write_fixture(
            "validate-yaml-source-unsafe-tag.yaml",
            r#"payload: !include secret.yaml
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/x-yaml",
                "--schema",
                cem_ml::schema::registry::YAML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.yaml.unsafe_tag"));
    }

    #[test]
    fn validate_csv_source_uses_csv_parser() {
        let p = write_fixture(
            "validate-csv-source.csv",
            "id,name,notes\n1,Ada,\"line one\nline two\"\n2,Lin,\"quoted \"\"value\"\"\"\n",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/csv; charset=utf-8; header=present",
                "--schema",
                cem_ml::schema::registry::CSV_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics.is_empty());
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_csv_source_reports_unclosed_quote_diagnostics() {
        let p = write_fixture(
            "validate-csv-source-unclosed-quote.csv",
            "id,name\n1,\"Ada\n",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/csv",
                "--schema",
                cem_ml::schema::registry::CSV_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.csv.unclosed_quote"));
    }

    #[test]
    fn validate_csv_source_reports_inconsistent_field_count_warning() {
        let p = write_fixture(
            "validate-csv-source-ragged.csv",
            "id,name,email\n1,Ada,ada@example.test\n2,Lin\n",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/csv",
                "--schema",
                cem_ml::schema::registry::CSV_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.csv.inconsistent_field_count"));
    }

    #[test]
    fn validate_csv_source_reports_unsupported_encoding() {
        let p = write_binary_fixture("validate-csv-source-invalid-utf8.csv", b"id,name\n1,\xff\n");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/csv",
                "--schema",
                cem_ml::schema::registry::CSV_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.csv.unsupported_encoding"));
    }

    #[test]
    fn validate_markdown_source_uses_markdown_parser() {
        let p = write_fixture(
            "validate-markdown-source.md",
            r#"# Release Notes

This document has **strong** text, [a link](https://example.test), and a list.

- Added schema validation.
- Kept source identity.
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/markdown; charset=utf-8; variant=CommonMark",
                "--schema",
                cem_ml::schema::registry::MARKDOWN_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics.is_empty());
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_markdown_source_reports_charset_missing_warning() {
        let p = write_fixture("validate-markdown-source-no-charset.md", "# Title\n");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/markdown",
                "--schema",
                cem_ml::schema::registry::MARKDOWN_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.markdown.charset_missing"));
    }

    #[test]
    fn validate_markdown_source_reports_unknown_variant_warning() {
        let p = write_fixture("validate-markdown-source-unknown-variant.md", "# Title\n");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/markdown; charset=utf-8; variant=CustomWiki",
                "--schema",
                cem_ml::schema::registry::MARKDOWN_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.markdown.unknown_variant"));
    }

    #[test]
    fn validate_markdown_source_reports_embedded_html_diagnostics() {
        let p = write_fixture(
            "validate-markdown-source-embedded-html.md",
            "# Unsafe\n\n<script>alert('x')</script>\n",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/markdown; charset=utf-8",
                "--schema",
                cem_ml::schema::registry::MARKDOWN_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.markdown.embedded_html_rejected"));
    }

    #[test]
    fn validate_markdown_source_reports_unsupported_encoding() {
        let p = write_binary_fixture("validate-markdown-source-invalid-utf8.md", b"# Bad\n\xff\n");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/markdown; charset=utf-8",
                "--schema",
                cem_ml::schema::registry::MARKDOWN_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.markdown.unsupported_encoding"));
    }

    #[test]
    fn validate_relax_ng_xml_source_uses_relax_ng_validator() {
        let p = write_fixture(
            "validate-relax-ng-source.rng",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start>
    <element name="note">
      <text/>
    </element>
  </start>
</grammar>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng+xml; charset=utf-8",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics.is_empty());
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn validate_relax_ng_compact_source_uses_relax_ng_validator() {
        let p = write_fixture(
            "validate-relax-ng-source.rnc",
            r#"default namespace = ""

start =
  element note {
    element title { text },
    element body { text }
  }
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng-compact-syntax",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn validate_relax_ng_xml_source_preserves_foreign_annotations() {
        let p = write_fixture(
            "validate-relax-ng-source-annotation.rng",
            r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0" xmlns:a="urn:annotation">
  <start>
    <element name="note">
      <a:documentation>Visible to schema tooling.</a:documentation>
      <text/>
    </element>
  </start>
</grammar>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng+xml",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn validate_relax_ng_xml_source_reports_missing_start() {
        let p = write_fixture(
            "validate-relax-ng-source-missing-start.rng",
            r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <define name="note">
    <element name="note"><text/></element>
  </define>
</grammar>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng+xml",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.relax_ng.missing_start"));
    }

    #[test]
    fn validate_relax_ng_xml_source_reports_unknown_element() {
        let p = write_fixture(
            "validate-relax-ng-source-unknown-element.rng",
            r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start>
    <element name="note"><unknown/></element>
  </start>
</grammar>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng+xml",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.relax_ng.unknown_element"));
    }

    #[test]
    fn validate_relax_ng_compact_source_reports_parse_error() {
        let p = write_fixture(
            "validate-relax-ng-source-invalid.rnc",
            r#"start =
  element note {
    element title { text }
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng-compact-syntax",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.relax_ng.compact_parse_error"));
    }

    #[test]
    fn validate_relax_ng_source_reports_external_reference_rejected() {
        let p = write_fixture(
            "validate-relax-ng-source-external.rng",
            r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start>
    <externalRef href="https://example.test/schema.rng"/>
  </start>
</grammar>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng+xml",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.relax_ng.external_ref_rejected"));
    }

    #[test]
    fn validate_relax_ng_source_reports_unsupported_encoding() {
        let p = write_binary_fixture(
            "validate-relax-ng-source-invalid-utf8.rng",
            b"<grammar>\xff",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/relax-ng+xml",
                "--schema",
                cem_ml::schema::registry::RELAX_NG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.relax_ng.unsupported_encoding"));
    }

    #[test]
    fn validate_xml_source_uses_xml_parser() {
        let p = write_fixture(
            "validate-xml-source.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog xmlns:meta="https://example.test/meta" meta:version="1">
  <item id="a1">Alpha</item>
  <item id="b2">Beta</item>
</catalog>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "text/xml; charset=utf-8",
                "--schema",
                cem_ml::schema::registry::XML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(diagnostics.is_empty());
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_xml_source_reports_parse_diagnostics() {
        let p = write_fixture("validate-xml-source-invalid.xml", "<root><item></root>\n");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/xml",
                "--schema",
                cem_ml::schema::registry::XML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.xml.parse_error"));
    }

    #[test]
    fn validate_xml_source_reports_unbound_namespace_prefix() {
        let p = write_fixture(
            "validate-xml-source-unbound-prefix.xml",
            "<root><meta:item/></root>\n",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/xml",
                "--schema",
                cem_ml::schema::registry::XML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.xml.unbound_namespace_prefix"));
    }

    #[test]
    fn validate_xml_source_reports_duplicate_attribute() {
        let p = write_fixture(
            "validate-xml-source-duplicate-attribute.xml",
            r#"<root xmlns:a="urn:test" xmlns:b="urn:test" a:id="1" b:id="2"/>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/xml",
                "--schema",
                cem_ml::schema::registry::XML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.xml.duplicate_attribute"));
    }

    #[test]
    fn validate_xml_source_reports_dtd_rejected() {
        let p = write_fixture(
            "validate-xml-source-doctype.xml",
            r#"<!DOCTYPE root SYSTEM "file:///etc/passwd">
<root/>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/xml",
                "--schema",
                cem_ml::schema::registry::XML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.xml.dtd_rejected"));
    }

    #[test]
    fn validate_xml_source_reports_external_entity_rejected() {
        let p = write_fixture(
            "validate-xml-source-external-entity.xml",
            r#"<root title="&secret;">safe &amp; text</root>
"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/xml",
                "--schema",
                cem_ml::schema::registry::XML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.xml.external_entity_rejected"));
    }

    #[test]
    fn validate_xml_source_reports_unsupported_encoding() {
        let p = write_binary_fixture("validate-xml-source-invalid-utf8.xml", b"<root>\xff</root>");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/xml",
                "--schema",
                cem_ml::schema::registry::XML_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.xml.unsupported_encoding"));
    }

    #[test]
    fn validate_json_schema_source_uses_json_schema_validator() {
        let p = write_fixture(
            "validate-json-schema-source.schema.json",
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "name": {
      "type": "string"
    }
  }
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/schema+json",
                "--schema",
                cem_ml::schema::registry::JSON_SCHEMA_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_json_schema_source_reports_unsupported_dialect() {
        let p = write_fixture(
            "validate-json-schema-source-unsupported-dialect.schema.json",
            r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object"
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                "application/schema+json",
                "--schema",
                cem_ml::schema::registry::JSON_SCHEMA_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.json_schema.unsupported_dialect"));
    }

    #[test]
    fn validate_cem_dom_binary_projection_source_uses_binary_validator() {
        let doc = test_cem_document("{p | Hi}");
        let artifact = cem_ml::projection::dom_binary_projection_artifact(&doc);
        let p = write_binary_fixture("validate-cem-dom-source.cem-bin", &artifact.bytes);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_cem_dom_binary_projection_source_reports_magic_diagnostic() {
        let p = write_binary_fixture("validate-cem-dom-source-invalid.cem-bin", b"not-cem-proj");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.projection.dom.binary_magic"));
    }

    #[test]
    fn validate_cem_dom_json_projection_source_uses_dom_json_validator() {
        let p = write_fixture(
            "validate-cem-dom-source.dom.json",
            r#"{
  "kind": "document",
  "children": [
    {
      "kind": "element",
      "name": "p",
      "namespace": "",
      "attributes": [],
      "children": [
        {
          "kind": "text",
          "data": "Hi",
          "byteRange": {
            "start": 5,
            "len": 2
          }
        }
      ],
      "byteRange": {
        "start": 0,
        "len": 8
      }
    }
  ]
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_cem_dom_json_projection_source_reports_shape_diagnostic() {
        let p = write_fixture(
            "validate-cem-dom-source-invalid.dom.json",
            r#"{
  "kind": "document",
  "children": [
    {
      "kind": "widget",
      "name": "p",
      "namespace": "",
      "attributes": [],
      "children": []
    }
  ]
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.projection.dom.json_shape"));
    }

    #[test]
    fn validate_cem_ast_binary_projection_source_uses_binary_validator() {
        let doc = test_cem_document("{p | Hi}");
        let artifact = cem_ml::projection::ast_binary_projection_artifact(&doc);
        let p = write_binary_fixture("validate-cem-ast-source.cem-bin", &artifact.bytes);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_AST_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_cem_ast_binary_projection_source_reports_magic_diagnostic() {
        let p = write_binary_fixture("validate-cem-ast-source-invalid.cem-bin", b"not-cem-proj");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_AST_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.projection.ast.binary_magic"));
    }

    #[test]
    fn validate_cem_ast_json_projection_source_uses_ast_json_validator() {
        let p = write_fixture(
            "validate-cem-ast-source.ast.json",
            r#"{
  "kind": "document",
  "children": [
    {
      "kind": "element",
      "name": "p",
      "namespace": "",
      "attributes": [],
      "children": [
        {
          "kind": "text",
          "data": "Hi",
          "byteRange": {
            "start": 5,
            "len": 2
          }
        }
      ],
      "byteRange": {
        "start": 0,
        "len": 8
      }
    }
  ]
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_cem_ast_json_projection_source_reports_shape_diagnostic() {
        let p = write_fixture(
            "validate-cem-ast-source-invalid.ast.json",
            r#"{
  "kind": "document",
  "children": [
    {
      "kind": "widget",
      "name": "p",
      "namespace": "",
      "attributes": [],
      "children": []
    }
  ]
}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_AST_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.projection.ast.json_shape"));
    }

    #[test]
    fn validate_cem_events_binary_projection_source_uses_binary_validator() {
        let artifact = cem_ml::projection::events_binary_projection_artifact_as(
            b"{p | Hi}",
            cem_ml::engine::InputFormat::Cem,
        );
        let p = write_binary_fixture("validate-cem-events-source.cem-bin", &artifact.bytes);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_cem_events_binary_projection_source_reports_magic_diagnostic() {
        let p = write_binary_fixture(
            "validate-cem-events-source-invalid.cem-bin",
            b"not-cem-proj",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.projection.events.binary_magic"));
    }

    #[test]
    fn validate_cem_events_json_projection_source_uses_events_json_validator() {
        let p = write_fixture(
            "validate-cem-events-source.events.json",
            r#"[
  {
    "kind": "open",
    "name": "p",
    "byteRange": {
      "start": 0,
      "len": 2
    }
  },
  {
    "kind": "separator",
    "byteRange": {
      "start": 3,
      "len": 1
    }
  },
  {
    "kind": "value",
    "value": "Hi",
    "byteRange": {
      "start": 5,
      "len": 2
    }
  },
  {
    "kind": "close",
    "name": "p",
    "byteRange": {
      "start": 7,
      "len": 1
    }
  }
]"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
        assert!(!diagnostics.iter().any(|diag| diag["code"]
            .as_str()
            .is_some_and(|code| code.starts_with("cem.schema."))));
    }

    #[test]
    fn validate_cem_events_json_projection_source_reports_shape_diagnostic() {
        let p = write_fixture(
            "validate-cem-events-source-invalid.events.json",
            r#"[
  {
    "kind": "widget",
    "byteRange": {
      "start": 0,
      "len": 1
    }
  }
]"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::schema::registry::CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                "--schema",
                cem_ml::schema::registry::CEM_EVENTS_PROJECTION_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.projection.events.json_shape"));
    }

    #[test]
    fn validate_html_schema_selects_html_input_adapter() {
        let p = write_fixture("validate-html-schema.data", "<p>Hi</p>");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--schema",
                "http://www.w3.org/1999/xhtml",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn validate_svg_schema_selects_html_input_adapter() {
        let p = write_fixture("validate-svg-schema.data", "<svg><title>Hi</title></svg>");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--schema",
                "http://www.w3.org/2000/svg",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn validate_unknown_input_namespace_reports_unsupported_adapter() {
        let p = write_fixture("validate-unknown-input-namespace.data", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--namespace",
                "widget=https://example.test/ns/widgets/1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|diag| {
            diag["code"] == "cem.lifecycle.adapter_unsupported"
                && diag["message"].as_str().is_some_and(|message| {
                    message.contains("namespace `widget=https://example.test/ns/widgets/1`")
                })
        }));
    }

    #[test]
    fn validate_html_namespace_selects_html_input_adapter() {
        let p = write_fixture("validate-html-namespace.data", "<p>Hi</p>");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--default-namespace",
                "http://www.w3.org/1999/xhtml",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn validate_xslt_namespace_selects_legacy_xslt_input_adapter() {
        let p = write_fixture(
            "validate-xslt-namespace.data",
            r#"<xsl:if test="$ready"><button>Go</button></xsl:if>"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--namespace",
                "xsl=http://www.w3.org/1999/XSL/Transform",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn validate_applies_base_uri_to_report_inputs_and_diagnostics() {
        let p = PathBuf::from("dist/target/cem_ml_cli/base-uri-input.cem");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "{unknown}").unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--base-uri",
                "file:///repo/",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let expected_uri = format!("file:///repo/{}", p.display());
        assert_eq!(v["inputs"][0], expected_uri);
        assert!(v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|diag| diag["uri"] == expected_uri));
    }

    #[test]
    fn validate_accepts_input_spec_without_positional_input() {
        let p = write_fixture("validate-input-spec.html", r#"<button>Go</button>"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!(
                    "uri={},contentType={}",
                    p.display(),
                    cem_ml::legacy_custom_element::TEMPLATE_LANG
                ),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
    }

    #[test]
    fn validate_input_spec_file_uri_reads_local_input() {
        let p = write_fixture("validate-input-spec-file-uri.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri=file://{}", p.display()),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
        assert_eq!(v["inputs"][0], format!("file://{}", p.display()));
    }

    #[test]
    fn validate_input_spec_remote_uri_input_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                "uri=https://example.test/input.cem",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "https://example.test/input.cem");
    }

    #[test]
    fn validate_positional_file_uri_reads_local_input() {
        let p = write_fixture("validate-positional-file-uri.cem", "{p Hi}");
        let file_uri = format!("file://{}", p.display());
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--format", "json", &file_uri],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
        assert_eq!(v["inputs"][0], file_uri);
    }

    #[test]
    fn validate_positional_remote_uri_input_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "https://example.test/input.cem",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "https://example.test/input.cem");
    }

    #[test]
    fn check_positional_remote_uri_input_is_rejected_without_resolver() {
        let uri = "https://example.test/check.cem";
        assert_remote_input_uri_rejected(&["check", "--format", "json", uri], uri);
    }

    #[test]
    fn check_positional_file_uri_input_reads_local_input() {
        let p = write_fixture("check-file-uri-input.cem", "{x}");
        let file_uri = local_file_uri(&p);
        let (outcome, stdout, stderr) = run(&FakeEngine, &["check", "--format", "json", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["inputs"][0], file_uri);
    }

    #[test]
    fn validate_config_remote_uri_input_is_rejected_without_resolver() {
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/validate-remote-uri-input-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": "https://example.test/input.cem"
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--config", config_path.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "https://example.test/input.cem");
    }

    #[test]
    fn validate_config_custom_uri_input_is_rejected_without_resolver() {
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/validate-custom-uri-input-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": "cem+vfs://workspace/input.cem"
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--config", config_path.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "cem+vfs://workspace/input.cem");
    }

    #[test]
    fn validate_config_non_local_file_uri_input_is_rejected() {
        let config_path = std::env::temp_dir()
            .join("cem-ml-cli-tests/validate-non-local-file-uri-input-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": "file://example.test/input.cem"
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--config", config_path.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_local_file_uri_boundary(&stderr, "file://example.test/input.cem");
    }

    #[test]
    fn input_spec_inherits_global_content_type_default() {
        let p = write_fixture("validate-input-spec-default.html", r#"<button>Go</button>"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--content-type",
                cem_ml::legacy_custom_element::TEMPLATE_LANG,
                "--input-spec",
                &format!("uri={}", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
    }

    #[test]
    fn input_spec_root_scope_fields_surface_execution_diagnostics() {
        let p = write_fixture("validate-input-spec-scope.cem", r#"{p Hi}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!(
                    "uri={},moduleMap=cem.modules.json,policy=strict,budgets=layoutMs:5",
                    p.display()
                ),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unenforced"));
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.policy_unenforced"));
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn scope_policy_context_option_surfaces_execution_diagnostic_for_positional_input() {
        let p = write_fixture("validate-context-scope-policy.cem", r#"{p Hi}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--scope-policy",
                "strict",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.policy_unenforced"));
    }

    #[test]
    fn input_spec_module_map_json_is_loaded() {
        let p = write_fixture(
            "validate-input-spec-module-map.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let module_map = write_fixture(
            "validate-input-spec-cem.modules.json",
            r#"{"schemas":{"ui/button":"./schemas/button.schema"}}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri={},moduleMap={}", p.display(), module_map.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unreadable"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_invalid"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unenforced"));
    }

    #[test]
    fn input_spec_module_map_file_uri_is_loaded() {
        let p = write_fixture(
            "validate-input-spec-module-map-file-uri.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let module_map = write_fixture(
            "validate-input-spec-file-uri-cem.modules.json",
            r#"{"schemas":{"ui/button":"./schemas/button.schema"}}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!(
                    "uri={},moduleMap=file://{}",
                    p.display(),
                    module_map.display()
                ),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unreadable"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_invalid"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unenforced"));
    }

    #[test]
    fn input_spec_module_map_localhost_file_uri_is_loaded() {
        let p = write_fixture(
            "validate-input-spec-module-map-localhost-file-uri.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let module_map = write_fixture(
            "validate-input-spec-localhost-file-uri-cem.modules.json",
            r#"{"schemas":{"ui/button":"./schemas/button.schema"}}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!(
                    "uri={},moduleMap={}",
                    p.display(),
                    localhost_file_uri(&module_map)
                ),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unreadable"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_invalid"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unenforced"));
    }

    #[test]
    fn input_spec_remote_module_map_uri_reports_unsupported_resolver() {
        let p = write_fixture(
            "validate-input-spec-remote-module-map.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!(
                    "uri={},moduleMap=https://example.test/cem.modules.json",
                    p.display()
                ),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|diag| {
            diag["code"] == "cem.scope.module_map_resolver_unsupported"
                && diag["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("remote/custom URI resolver"))
        }));
        assert!(!diagnostics.iter().any(|diag| diag["message"]
            .as_str()
            .is_some_and(|message| message.contains("No such file"))));
    }

    #[test]
    fn input_spec_non_local_file_uri_module_map_reports_unreadable() {
        let p = write_fixture(
            "validate-input-spec-non-local-module-map.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let module_map = "file://example.test/cem.modules.json";
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri={},moduleMap={module_map}", p.display()),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|diag| {
            diag["code"] == "cem.scope.module_map_unreadable"
                && diag["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("only local file://"))
        }));
        assert!(diagnostics.iter().any(|diag| diag["message"]
            .as_str()
            .is_some_and(|message| message.contains(module_map))));
    }

    #[test]
    fn module_map_context_option_is_loaded_for_positional_input() {
        let p = write_fixture(
            "validate-context-module-map.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let module_map = write_fixture(
            "validate-context-cem.modules.json",
            r#"{"schemas":{"ui/button":"./schemas/button.schema"}}"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--module-map",
                module_map.to_str().unwrap(),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unreadable"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_invalid"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unenforced"));
    }

    #[test]
    fn module_map_context_option_reports_resolver_boundary_diagnostics() {
        let remote = write_fixture(
            "validate-context-remote-module-map.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let custom = write_fixture(
            "validate-context-custom-module-map.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let non_local = write_fixture(
            "validate-context-non-local-module-map.cem",
            r#"@schema src="ui/button"
{p Hi}"#,
        );
        let cases = [
            (
                "https://example.test/cem.modules.json",
                remote.to_str().unwrap(),
                "cem.scope.module_map_resolver_unsupported",
                "remote/custom URI resolver",
            ),
            (
                "cem+vfs://workspace/cem.modules.json",
                custom.to_str().unwrap(),
                "cem.scope.module_map_resolver_unsupported",
                "remote/custom URI resolver",
            ),
            (
                "file://example.test/cem.modules.json",
                non_local.to_str().unwrap(),
                "cem.scope.module_map_unreadable",
                "only local file://",
            ),
        ];

        for (module_map, input, code, message) in cases {
            let (outcome, stdout, stderr) = run(
                &RealCemMlEngine::new(),
                &[
                    "validate",
                    "--format",
                    "json",
                    "--module-map",
                    module_map,
                    input,
                ],
            );

            assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
            let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
            let diagnostics = v["diagnostics"].as_array().unwrap();
            assert!(diagnostics.iter().any(|diag| {
                diag["code"] == code
                    && diag["message"]
                        .as_str()
                        .is_some_and(|text| text.contains(message))
            }));
        }
    }

    #[test]
    fn input_spec_version_pins_resolve_through_engine() {
        let p = write_fixture("validate-input-spec-version-pin.cem", r#"{p Hi}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri={},versionPins=cem-ml:1", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.doc.version_resolved"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.version_pins_unenforced"));
    }

    #[test]
    fn version_pin_context_option_resolves_for_positional_input() {
        let p = write_fixture("validate-context-version-pin.cem", r#"{p Hi}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--version-pin",
                "cem-ml=1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.doc.version_resolved"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.version_pins_unenforced"));
    }

    #[test]
    fn input_spec_parse_ms_budget_is_enforced() {
        let p = write_fixture("validate-input-spec-parse-budget.cem", r#"{p Hi}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri={},budgets=parseMs:0", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["inputCount"], 1);
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
    }

    #[test]
    fn input_spec_validate_ms_budget_is_enforced() {
        let p = write_fixture("validate-input-spec-validate-budget.cem", r#"{p Hi}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri={},budgets=validateMs:0", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn positional_input_identity_infers_content_type_from_extension() {
        let identity =
            positional_input_scope(Path::new("src/screen.html"), &ScopeConfig::default())
                .format_identity_option()
                .expect("html extension should infer content type");
        assert_eq!(identity.content_type.as_deref(), Some("text/html"));
        assert_eq!(identity.schema, None);
        assert_eq!(identity.base_uri, None);

        let svg_identity =
            positional_input_scope(Path::new("src/icon.svg"), &ScopeConfig::default())
                .format_identity_option()
                .expect("svg extension should infer content type");
        assert_eq!(svg_identity.content_type.as_deref(), Some("image/svg+xml"));
        assert_eq!(svg_identity.schema, None);
        assert_eq!(svg_identity.base_uri, None);

        let xhtml_identity =
            positional_input_scope(Path::new("src/screen.xhtml"), &ScopeConfig::default())
                .format_identity_option()
                .expect("xhtml extension should infer content type");
        assert_eq!(
            xhtml_identity.content_type.as_deref(),
            Some("application/xhtml+xml")
        );
        assert_eq!(xhtml_identity.schema, None);
        assert_eq!(xhtml_identity.base_uri, None);
    }

    #[test]
    fn positional_input_scope_carries_context_namespace_defaults() {
        let context = cli::ContextOptions {
            schema: Some("https://cem.dev/ns/core/1".to_owned()),
            content_type: None,
            default_namespace: Some("urn:default".to_owned()),
            namespaces: vec![cli::NamespaceBinding {
                prefix: "widget".to_owned(),
                uri: "urn:widgets".to_owned(),
            }],
            module_map: Some("cem.modules.json".to_owned()),
            version_pins: vec![cli::ScopeKeyValue {
                key: "cem-ml".to_owned(),
                value: "1".to_owned(),
            }],
            scope_policy: Some("deterministic".to_owned()),
            scope_budgets: vec![cli::ScopeKeyValue {
                key: "parseMs".to_owned(),
                value: "5".to_owned(),
            }],
            base_uri: Some("file:///workspace/".to_owned()),
            ..cli::ContextOptions::default()
        };

        let defaults = input_scope_defaults(&context);
        let scope = positional_input_scope(Path::new("src/screen.html"), &defaults);

        assert_eq!(scope.default_content_type.as_deref(), Some("text/html"));
        assert_eq!(scope.schema.as_deref(), Some("https://cem.dev/ns/core/1"));
        assert_eq!(scope.default_namespace.as_deref(), Some("urn:default"));
        assert_eq!(
            scope.namespaces.get("widget").map(String::as_str),
            Some("urn:widgets")
        );
        assert_eq!(scope.module_map.as_deref(), Some("cem.modules.json"));
        assert_eq!(
            scope.version_pins.get("cem-ml").map(String::as_str),
            Some("1")
        );
        assert_eq!(scope.policy.as_deref(), Some("deterministic"));
        assert_eq!(scope.budgets.get("parseMs").map(String::as_str), Some("5"));
        assert_eq!(scope.base_uri.as_deref(), Some("file:///workspace/"));
    }

    #[test]
    fn validate_positional_html_uses_inferred_content_type_identity() {
        let p = write_fixture("validate-positional-html.html", r#"<button>Go</button>"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--format", "json", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
    }

    #[test]
    fn validate_positional_xhtml_uses_inferred_html_input_adapter() {
        let p = write_fixture("validate-positional-xhtml.xhtml", r#"<button>Go</button>"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--format", "json", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
        assert!(!v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diag| { diag["code"] == "cem.lifecycle.adapter_unsupported" }));
    }

    #[test]
    fn validate_positional_svg_uses_inferred_xml_input_adapter() {
        let p = write_fixture(
            "validate-positional-svg.svg",
            r#"<svg><title>Download</title></svg>"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--format", "json", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
        assert!(!v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diag| { diag["code"] == "cem.lifecycle.adapter_unsupported" }));
    }

    #[test]
    fn convert_config_input_and_output_specs_select_identity_and_destination() {
        let input = write_fixture(
            "convert-config-input.html",
            r#"<if test="$ready"><button>Go</button></if>"#,
        );
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-config-out.json");
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-config.json");
        let _ = std::fs::remove_file(&out_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string(),
                    "rootScope": {
                        "defaultContentType": cem_ml::legacy_custom_element::TEMPLATE_LANG
                    }
                }],
                "outputs": [{
                    "destination": out_path.display().to_string(),
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "cem");
        assert_eq!(v["content"], "{cem:if @test=\"ready\" | {button | Go}}\n");
    }

    #[test]
    fn convert_config_relative_destination_is_resolved_against_config_path() {
        let input = write_fixture("convert-config-relative-dest-input.cem", "{p Hi}");
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/convert-relative-dest");
        let config_path = dir.join("run.json");
        let out_path = dir.join("dist/out.json");
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(&out_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string()
                }],
                "outputs": [{
                    "destination": "dist/out.json",
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["content"], "{p Hi}\n");
    }

    #[test]
    fn convert_file_uri_config_resolves_relative_destination_against_config_path() {
        let input = write_fixture("convert-file-uri-config-relative-dest-input.cem", "{p Hi}");
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/convert-file-uri-config");
        let config_path = dir.join("run.json");
        let out_path = dir.join("dist/out.json");
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(&out_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string()
                }],
                "outputs": [{
                    "destination": "dist/out.json",
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let config_uri = format!("file://{}", config_path.display());
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", &config_uri],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["content"], "{p Hi}\n");
    }

    #[test]
    fn convert_localhost_file_uri_config_resolves_relative_destination_against_config_path() {
        let input = write_fixture(
            "convert-localhost-file-uri-config-relative-dest-input.cem",
            "{p Hi}",
        );
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/convert-localhost-file-uri-config");
        let config_path = dir.join("run.json");
        let out_path = dir.join("dist/out.json");
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(&out_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string()
                }],
                "outputs": [{
                    "destination": "dist/out.json",
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let config_uri = localhost_file_uri(&config_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", &config_uri],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["content"], "{p Hi}\n");
    }

    #[test]
    fn convert_config_file_uri_destination_writes_local_output() {
        let input = write_fixture("convert-config-file-uri-dest-input.cem", "{p Hi}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-file-uri-dest.json");
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-file-uri-dest-config.json");
        let _ = std::fs::remove_file(&out_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string()
                }],
                "outputs": [{
                    "destination": format!("file://{}", out_path.display()),
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["content"], "{p Hi}\n");
    }

    #[test]
    fn convert_config_remote_uri_destination_is_rejected() {
        let input = write_fixture("convert-config-remote-uri-dest-input.cem", "{p Hi}");
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-remote-uri-dest-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string()
                }],
                "outputs": [{
                    "destination": "https://example.test/out.json",
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "https://example.test/out.json");
    }

    #[test]
    fn convert_config_custom_uri_destination_is_rejected() {
        let input = write_fixture("convert-config-custom-uri-dest-input.cem", "{p Hi}");
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-custom-uri-dest-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string()
                }],
                "outputs": [{
                    "destination": "cem+vfs://workspace/out.json",
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "cem+vfs://workspace/out.json");
    }

    #[test]
    fn convert_config_non_local_file_uri_destination_is_rejected() {
        let input = write_fixture("convert-config-non-local-file-uri-dest-input.cem", "{p Hi}");
        let config_path = std::env::temp_dir()
            .join("cem-ml-cli-tests/convert-non-local-file-uri-dest-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string()
                }],
                "outputs": [{
                    "destination": "file://example.test/out.json",
                    "rootScope": {
                        "defaultContentType": "application/cem+xml"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_local_file_uri_boundary(&stderr, "file://example.test/out.json");
    }

    #[test]
    fn local_file_uri_output_path_decodes_percent_escaped_paths() {
        assert_eq!(
            local_file_uri_to_path("file:///tmp/cem%20ml/out.json").unwrap(),
            PathBuf::from("/tmp/cem ml/out.json")
        );
        assert_eq!(
            local_file_uri_to_path("file://localhost/tmp/cem%23ml/out.json").unwrap(),
            PathBuf::from("/tmp/cem#ml/out.json")
        );
        assert!(local_file_uri_to_path("file://example.test/tmp/out.json").is_none());
    }

    #[test]
    fn local_file_uri_output_path_rejects_malformed_percent_escapes() {
        assert!(local_file_uri_to_path("file:///tmp/cem%2/out.json").is_none());
        assert!(local_file_uri_to_path("file:///tmp/cem%zz/out.json").is_none());
    }

    #[test]
    fn local_path_or_file_uri_rejects_remote_or_custom_uri_schemes() {
        let err = local_path_or_file_uri(
            Path::new("https://example.test/out.json"),
            "output destination",
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err
            .to_string()
            .contains("remote/custom URI resolvers are not implemented"));

        assert!(
            local_path_or_file_uri(Path::new("relative/out.json"), "output destination").is_ok()
        );
    }

    #[test]
    fn local_path_or_file_uri_rejects_non_local_file_uri_hosts() {
        let err = local_path_or_file_uri(
            Path::new("file://example.test/out.json"),
            "output destination",
        )
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err
            .to_string()
            .contains("only local file:// URIs are supported"));
    }

    #[test]
    fn custom_uri_primary_output_uses_registered_write_resolver() {
        let (context, writes) = context_with_write_resolver(ResolvePurpose::Output);
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let mut streams = Streams {
            stdout: &mut stdout,
            stderr: &mut stderr,
            quiet: false,
        };

        write_primary(
            &context,
            &serde_json::json!({"ok": true}),
            Some(Path::new("cem+vfs://out/result.json")),
            &mut streams,
        )
        .unwrap();

        let writes = writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, "cem+vfs://out/result.json");
        assert!(String::from_utf8_lossy(&writes[0].1).contains(r#""ok": true"#));
    }

    #[test]
    fn convert_primary_writer_prefers_native_primary_bytes() {
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let mut streams = Streams {
            stdout: &mut stdout,
            stderr: &mut stderr,
            quiet: false,
        };
        let response = eng::ConvertResponse {
            primary: serde_json::json!({"kind": "not-binary-envelope"}),
            primary_bytes: Some(eng::PrimaryBytes {
                content_type: "application/vnd.cem.dom+cem-bin".to_owned(),
                schema: Some("https://cem.dev/ns/projection/dom/1".to_owned()),
                format_version: "cem-projection-bin/1".to_owned(),
                hash_scheme: "cem-bin/1+blake3".to_owned(),
                hash: "cem-bin/1+blake3:test".to_owned(),
                bytes: b"CEMPROJ\0native".to_vec(),
            }),
            diagnostics: Vec::new(),
            scheduler_trace: cem_ml::report::SchedulerTraceReport::default(),
        };

        write_convert_primary(
            &eng::EngineContext::default(),
            &response,
            None,
            &mut streams,
        )
        .unwrap();

        assert_eq!(stdout.into_inner(), b"CEMPROJ\0native");
    }

    #[test]
    fn binary_projection_metadata_primary_writes_json_without_chunks() {
        let mut stdout = Cursor::new(Vec::new());
        let mut stderr = Cursor::new(Vec::new());
        let mut streams = Streams {
            stdout: &mut stdout,
            stderr: &mut stderr,
            quiet: false,
        };

        write_primary(
            &eng::EngineContext::default(),
            &serde_json::json!({
                "kind": "cem-binary-projection",
                "projection": "dom",
                "contentType": "application/vnd.cem.dom+cem-bin",
                "hash": "cem-bin/1+blake3:test",
                "nativeBytes": true
            }),
            None,
            &mut streams,
        )
        .unwrap();

        let written = String::from_utf8(stdout.into_inner()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "cem-binary-projection");
        assert!(v.get("chunks").is_none());
    }

    #[test]
    fn custom_uri_input_uses_registered_read_resolver() {
        let context = context_with_read_resolver(
            ResolvePurpose::Input,
            StaticReadResolver {
                uri: "cem+vfs://inputs/source.cem",
                bytes: b"{main | Loaded}",
                content_type: Some("application/cem+xml"),
            },
        );

        let input = engine_input(
            &context,
            Path::new("cem+vfs://inputs/source.cem"),
            None,
            ScopeConfig::default(),
        )
        .unwrap();

        assert_eq!(input.uri, "cem+vfs://inputs/source.cem");
        assert_eq!(input.bytes, b"{main | Loaded}");
        assert_eq!(
            input.root_scope.default_content_type.as_deref(),
            Some("application/cem+xml")
        );
    }

    #[test]
    fn custom_uri_config_uses_registered_read_resolver() {
        let context = context_with_read_resolver(
            ResolvePurpose::Config,
            StaticReadResolver {
                uri: "cem+vfs://configs/run.json",
                bytes: br#"{"inputs":[{"uri":"local.cem"}]}"#,
                content_type: Some("application/json"),
            },
        );
        let config = match run_config_with_context(
            &context,
            &cli::RunOptions {
                config: Some(PathBuf::from("cem+vfs://configs/run.json")),
                ..cli::RunOptions::default()
            },
            RunConfigDefaults::default(),
        ) {
            Ok(config) => config,
            Err(_) => panic!("custom resolver config should parse"),
        };

        assert_eq!(config.inputs.len(), 1);
        assert_eq!(config.inputs[0].uri, "local.cem");
    }

    #[test]
    fn resolver_read_map_loads_custom_config_and_input() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/resolver-read-map");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("input.cem"), "{p | Loaded}").unwrap();
        std::fs::write(
            root.join("run.json"),
            serde_json::json!({
                "inputs": [{
                    "uri": "cem+vfs://workspace/input.cem"
                }]
            })
            .to_string(),
        )
        .unwrap();

        let map = format!("cem+vfs://workspace={}", root.display());
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "validate",
                "--format",
                "json",
                "--resolver-read-map",
                &map,
                "--config",
                "cem+vfs://workspace/run.json",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stderr.trim().is_empty());
        assert!(stdout.contains("cem+vfs://workspace/input.cem"));
    }

    #[test]
    fn run_config_resolver_spec_loads_custom_input() {
        let root = std::env::temp_dir().join("cem-ml-cli-tests/config-resolver-read-map");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("input.cem"), "{p | Loaded}").unwrap();
        let config_path = write_fixture(
            "config-resolver-read-map.json",
            &serde_json::json!({
                "resolvers": [{
                    "uriPrefix": "cem+vfs://workspace",
                    "localRoot": root.display().to_string(),
                    "read": true
                }],
                "inputs": [{
                    "uri": "cem+vfs://workspace/input.cem"
                }]
            })
            .to_string(),
        );

        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "validate",
                "--format",
                "json",
                "--config",
                config_path.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stderr.trim().is_empty());
        assert!(stdout.contains("cem+vfs://workspace/input.cem"));
    }

    #[test]
    fn resolver_write_map_writes_custom_primary_output() {
        let input = write_fixture("resolver-write-map-input.cem", "{p | Loaded}");
        let root = std::env::temp_dir().join("cem-ml-cli-tests/resolver-write-map");
        let out_path = root.join("out/result.json");
        let _ = std::fs::remove_file(&out_path);
        let map = format!("cem+vfs://workspace={}", root.display());

        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "parse",
                "--resolver-write-map",
                &map,
                "--out",
                "cem+vfs://workspace/out/result.json",
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(out_path).unwrap();
        assert!(written.contains(r#""kind": "fake-parse""#));
    }

    #[test]
    fn fixture_placeholder_uses_registered_read_resolver() {
        let context = context_with_read_resolver(
            ResolvePurpose::Input,
            StaticReadResolver {
                uri: "cem+vfs://fixtures/source.cem",
                bytes: b"{main | Loaded}",
                content_type: Some("application/cem+xml"),
            },
        );
        let input = placeholder_input(
            Path::new("cem+vfs://fixtures/source.cem"),
            Some(cli::InputFormat::Cem),
            ScopeConfig::default(),
        );

        let materialized = materialize_fixture_input(&context, &input).unwrap();

        assert_eq!(materialized.uri, "cem+vfs://fixtures/source.cem");
        assert_eq!(materialized.bytes, b"{main | Loaded}");
        assert_eq!(
            materialized.root_scope.default_content_type.as_deref(),
            Some("application/cem+xml")
        );
    }

    #[test]
    fn custom_uri_report_destination_uses_registered_write_resolver() {
        let (context, writes) = context_with_write_resolver(ResolvePurpose::Report);

        write_destination(
            &context,
            Path::new("cem+vfs://reports/cem-ml.report.json"),
            "report destination",
            ResolvePurpose::Report,
            br#"{"summary":{}}"#,
        )
        .unwrap();

        let writes = writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, "cem+vfs://reports/cem-ml.report.json");
        assert_eq!(writes[0].1, br#"{"summary":{}}"#);
    }

    #[test]
    fn convert_config_fans_out_multiple_outputs() {
        let first = write_fixture("convert-fanout-first.cem", "{p First}");
        let second = write_fixture("convert-fanout-second.cem", "{p Second}");
        let first_out = std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-first.json");
        let second_out = std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-second.json");
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout.json");
        let _ = std::fs::remove_file(&first_out);
        let _ = std::fs::remove_file(&second_out);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [
                    { "uri": first.display().to_string() },
                    { "uri": second.display().to_string() }
                ],
                "outputs": [
                    {
                        "inputRef": first.display().to_string(),
                        "destination": first_out.display().to_string(),
                        "rootScope": { "defaultContentType": "application/cem+xml" }
                    },
                    {
                        "inputRef": second.display().to_string(),
                        "destination": second_out.display().to_string(),
                        "rootScope": { "defaultContentType": "application/cem+xml" }
                    }
                ]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());

        let first_written = std::fs::read_to_string(&first_out).unwrap();
        let first_json: serde_json::Value = serde_json::from_str(&first_written).unwrap();
        assert_eq!(first_json["content"], "{p First}\n");
        let second_written = std::fs::read_to_string(&second_out).unwrap();
        let second_json: serde_json::Value = serde_json::from_str(&second_written).unwrap();
        assert_eq!(second_json["content"], "{p Second}\n");
    }

    #[test]
    fn convert_writes_side_report_with_scheduler_trace() {
        let input = write_fixture("convert-report-input.cem", "{p Hi}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-report-output.json");
        let report_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-report.json");
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&report_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--out",
                out_path.to_str().unwrap(),
                "--report-json",
                report_path.to_str().unwrap(),
                input.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        assert!(out_path.is_file());
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        assert_eq!(report["summary"]["inputCount"], 1);
        assert_eq!(
            report["reportAst"]["schedulerTrace"]["events"][0]["task"],
            format!("{}:lifecycle-load", input.display())
        );
        assert!(report["reportAst"]["schedulerTrace"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["task"] == format!("{}:convert", input.display())));
    }

    #[test]
    fn convert_file_uri_out_destination_writes_local_output() {
        let input = write_fixture("convert-file-uri-out.cem", "{p Hi}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-file-uri-out.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "convert",
                "--out",
                &format!("file://{}", out_path.display()),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-convert");
    }

    #[test]
    fn convert_remote_uri_input_is_rejected_without_resolver() {
        let uri = "https://example.test/convert.cem";
        assert_remote_input_uri_rejected(&["convert", uri], uri);
    }

    #[test]
    fn convert_file_uri_input_reads_local_input() {
        let input = write_fixture("convert-file-uri-input.cem", "{p Hi}");
        let file_uri = local_file_uri(&input);
        let (outcome, stdout, stderr) = run(&FakeEngine, &["convert", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["input"], file_uri);
    }

    #[test]
    fn convert_remote_uri_out_destination_is_rejected_without_resolver() {
        let input = write_fixture("convert-remote-out.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "convert",
                "--out",
                "https://example.test/convert.json",
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/convert.json");
    }

    #[test]
    fn convert_remote_uri_markdown_report_destination_is_rejected_without_resolver() {
        let input = write_fixture("convert-remote-md-report-input.cem", "{p Hi}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-remote-md-report-output.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--out",
                out_path.to_str().unwrap(),
                "--report-md",
                "https://example.test/convert.md",
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert!(out_path.is_file());
        assert_stderr_contains_all(&stderr, &["convert report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/convert.md");
    }

    #[test]
    fn convert_custom_uri_report_destination_is_rejected_without_resolver() {
        let input = write_fixture("convert-custom-report-input.cem", "{p Hi}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-custom-report-output.json");
        let _ = std::fs::remove_file(&out_path);
        let uri = "cem+vfs://workspace/convert-report.json";
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--out",
                out_path.to_str().unwrap(),
                "--report-json",
                uri,
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert!(out_path.is_file());
        assert_stderr_contains_all(&stderr, &["convert report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn convert_custom_uri_markdown_report_destination_is_rejected_without_resolver() {
        let input = write_fixture("convert-custom-md-report-input.cem", "{p Hi}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-custom-md-report-output.json");
        let _ = std::fs::remove_file(&out_path);
        let uri = "cem+vfs://workspace/convert-report.md";
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--out",
                out_path.to_str().unwrap(),
                "--report-md",
                uri,
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert!(out_path.is_file());
        assert_stderr_contains_all(&stderr, &["convert report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn convert_non_local_file_uri_markdown_report_destination_is_rejected() {
        let input = write_fixture("convert-non-local-file-uri-md-report-input.cem", "{p Hi}");
        let out_path = std::env::temp_dir()
            .join("cem-ml-cli-tests/convert-non-local-file-uri-md-report-output.json");
        let _ = std::fs::remove_file(&out_path);
        let uri = "file://example.test/convert-report.md";
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--out",
                out_path.to_str().unwrap(),
                "--report-md",
                uri,
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert!(out_path.is_file());
        assert_stderr_contains_all(&stderr, &["convert report write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn output_spec_convert_ms_budget_is_reported() {
        let input = write_fixture("convert-output-spec-budget.cem", "{p Hi}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-output-spec-budget.json");
        let report_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-output-spec-budget-report.json");
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&report_path);

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--output-spec",
                &format!("dest={},budgets=convertMs:0", out_path.display()),
                "--report-json",
                report_path.to_str().unwrap(),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        assert!(out_path.is_file());
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        let diagnostics = report["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn convert_fanout_writes_side_report_for_each_output() {
        let first = write_fixture("convert-fanout-report-first.cem", "{p First}");
        let second = write_fixture("convert-fanout-report-second.cem", "{p Second}");
        let first_out =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-report-first.json");
        let second_out =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-report-second.json");
        let report_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-report.json");
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-report-config.json");
        let _ = std::fs::remove_file(&first_out);
        let _ = std::fs::remove_file(&second_out);
        let _ = std::fs::remove_file(&report_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [
                    { "uri": first.display().to_string() },
                    { "uri": second.display().to_string() }
                ],
                "outputs": [
                    {
                        "inputRef": first.display().to_string(),
                        "destination": first_out.display().to_string(),
                        "rootScope": { "defaultContentType": "application/cem+xml" }
                    },
                    {
                        "inputRef": second.display().to_string(),
                        "destination": second_out.display().to_string(),
                        "rootScope": { "defaultContentType": "application/cem+xml" }
                    }
                ],
                "scheduler": {
                    "maxParallelDocuments": 2
                }
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--config",
                config_path.to_str().unwrap(),
                "--report-json",
                report_path.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        assert_eq!(report["summary"]["inputCount"], 2);
        assert_eq!(report["reportAst"]["schedulerTrace"]["eventCount"], 18);
        assert_eq!(
            report["reportAst"]["schedulerTrace"]["events"][9]["scopeId"],
            1
        );
        assert!(first_out.is_file());
        assert!(second_out.is_file());
    }

    #[test]
    fn convert_config_multi_input_output_requires_input_ref() {
        let first = write_fixture("convert-fanout-ambiguous-first.cem", "{p First}");
        let second = write_fixture("convert-fanout-ambiguous-second.cem", "{p Second}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-ambiguous.json");
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-fanout-ambiguous.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [
                    { "uri": first.display().to_string() },
                    { "uri": second.display().to_string() }
                ],
                "outputs": [{ "destination": out_path.display().to_string() }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, _, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("requires `input`"));
    }

    #[test]
    fn output_spec_inherits_convert_target_identity_default() {
        let input = write_fixture("convert-output-default-input.cem", "{p Hi}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-output-default.cem");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-content-type",
                "application/cem+xml",
                "--output-spec",
                &format!("dest={}", out_path.display()),
                input.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "cem");
        assert_eq!(v["content"], "{p Hi}\n");
    }

    #[test]
    fn output_spec_file_uri_destination_writes_local_output() {
        let input = write_fixture("convert-output-file-uri-destination.cem", "{p Hi}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-output-file-uri-destination.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "convert",
                "--output-spec",
                &format!("dest={}", local_file_uri(&out_path)),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-convert");
    }

    #[test]
    fn output_spec_localhost_file_uri_destination_writes_local_output() {
        let input = write_fixture(
            "convert-output-localhost-file-uri-destination.cem",
            "{p Hi}",
        );
        let out_path = std::env::temp_dir()
            .join("cem-ml-cli-tests/convert-output-localhost-file-uri-destination.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "convert",
                "--output-spec",
                &format!("dest={}", localhost_file_uri(&out_path)),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-convert");
    }

    #[test]
    fn output_spec_remote_uri_destination_is_rejected_without_resolver() {
        let input = write_fixture("convert-output-remote-destination.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--output-spec",
                "dest=https://example.test/out.json",
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "https://example.test/out.json");
    }

    #[test]
    fn output_spec_custom_uri_destination_is_rejected_without_resolver() {
        let input = write_fixture("convert-output-custom-destination.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "convert",
                "--output-spec",
                "dest=cem+vfs://workspace/out.json",
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "cem+vfs://workspace/out.json");
    }

    #[test]
    fn output_spec_non_local_file_uri_destination_is_rejected() {
        let input = write_fixture(
            "convert-output-non-local-file-uri-destination.cem",
            "{p Hi}",
        );
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "convert",
                "--output-spec",
                "dest=file://example.test/out.json",
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_local_file_uri_boundary(&stderr, "file://example.test/out.json");
    }

    #[test]
    fn output_spec_namespace_identity_selects_cem_export_adapter() {
        let input = write_fixture("convert-output-namespace-input.cem", "{p Hi}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-output-namespace.out");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--output-spec",
                &format!(
                    "dest={},defaultNs=https://cem.dev/ns/core/1",
                    out_path.display()
                ),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "cem");
        assert_eq!(v["content"], "{p Hi}\n");
    }

    #[test]
    fn output_spec_html_namespace_identity_selects_html_export_adapter() {
        let input = write_fixture("convert-output-html-namespace-input.cem", "{p Hi}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-output-html-namespace.out");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--output-spec",
                &format!(
                    "dest={},defaultNs=http://www.w3.org/1999/xhtml",
                    out_path.display()
                ),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn output_spec_html_schema_identity_selects_html_export_adapter() {
        let input = write_fixture("convert-output-html-schema-input.cem", "{p Hi}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-output-html-schema.out");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--output-spec",
                &format!(
                    "dest={},schema=http://www.w3.org/1999/xhtml",
                    out_path.display()
                ),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn output_spec_xml_content_type_selects_xml_export_adapter() {
        let input = write_fixture(
            "convert-output-xml-content-type-input.cem",
            "{p @id=one | Hi}",
        );
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-output-xml-content-type.out");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--output-spec",
                &format!("dest={},contentType=application/xml", out_path.display()),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "xml");
        assert_eq!(v["content"], r#"<p id="one">Hi</p>"#);
    }

    #[test]
    fn output_spec_svg_destination_infers_svg_export_adapter() {
        let input = write_fixture(
            "convert-output-svg-destination-input.cem",
            "{svg | {title | Hi}}",
        );
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-output-svg-destination.svg");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "cem",
                "--output-spec",
                &format!("dest={}", out_path.display()),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "xml");
        assert_eq!(v["content"], "<svg><title>Hi</title></svg>");
    }

    #[test]
    fn output_spec_xhtml_destination_infers_html_export_adapter() {
        let input = write_fixture("convert-output-xhtml-destination-input.cem", "{p | Hi}");
        let out_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-output-xhtml-destination.xhtml");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "cem",
                "--output-spec",
                &format!("dest={}", out_path.display()),
                input.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn output_spec_projection_schema_identities_select_export_adapters() {
        let input = write_fixture("convert-output-projection-schema-input.cem", "{p | Hi}");

        for (name, schema) in [
            ("dom-json", cem_ml::lifecycle::DOM_JSON_PROJECTION_SCHEMA),
            ("ast", cem_ml::lifecycle::AST_PROJECTION_SCHEMA),
            ("events", cem_ml::lifecycle::EVENTS_PROJECTION_SCHEMA),
        ] {
            let out_path =
                std::env::temp_dir().join(format!("cem-ml-cli-tests/convert-output-{name}.json"));
            let _ = std::fs::remove_file(&out_path);
            let (outcome, stdout, stderr) = run(
                &RealCemMlEngine::new(),
                &[
                    "convert",
                    "--to-format",
                    "cem",
                    "--output-spec",
                    &format!(
                        "dest={},contentType=application/json,schema={schema}",
                        out_path.display()
                    ),
                    input.to_str().unwrap(),
                ],
            );

            assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
            assert!(stdout.trim().is_empty());
            assert!(
                !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
                "{stderr}"
            );
            let written = std::fs::read_to_string(&out_path).unwrap();
            let v: serde_json::Value = serde_json::from_str(&written).unwrap();
            match name {
                "events" => {
                    assert!(v.is_array());
                    assert!(v
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|event| event["kind"] == "open" && event["name"] == "p"));
                }
                _ => {
                    assert_eq!(v["kind"], "document");
                    assert_eq!(v["children"][0]["name"], "p");
                }
            }
        }
    }

    #[test]
    fn config_diagnostics_fail_before_document_parsing() {
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/bad-config.json");
        let report_path = std::env::temp_dir().join("cem-ml-cli-tests/bad-config-report.json");
        let _ = std::fs::remove_file(&report_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{ "uri": "/definitely/not/read.cem" }],
                "outputs": [{ "inputRef": "missing.cem" }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--config",
                config_path.to_str().unwrap(),
                "--report-json",
                report_path.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("cem.run_config.output_input_ref_unknown"));
        assert!(
            !stderr.contains("I/O error"),
            "config diagnostics must fail before input files are read: {stderr}"
        );
        let stdout_report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            stdout_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
        assert_eq!(stdout_report["summary"]["fatalCount"], 1);
        let written = std::fs::read_to_string(&report_path).unwrap();
        let file_report: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(
            file_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
    }

    #[test]
    fn run_config_schema_identity_is_accepted() {
        let input = write_fixture("run-config-schema-input.cem", "{p | Hi}");
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/run-config-schema.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{ "uri": input.display().to_string() }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--config",
                config_path.to_str().unwrap(),
                "--config-schema",
                run_config::RUN_CONFIG_SCHEMA_URI,
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stderr.trim().is_empty(), "{stderr}");
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(report["summary"]["fatalCount"], 0);
        assert!(!report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diag| {
                diag["code"]
                    .as_str()
                    .is_some_and(|code| code.starts_with("cem.run_config."))
            }));
    }

    #[test]
    fn run_config_unknown_schema_fails_before_document_parsing() {
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/run-config-unknown-schema.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{ "uri": "/definitely/not/read.cem" }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--config",
                config_path.to_str().unwrap(),
                "--config-schema",
                "https://cem.dev/ns/core/1",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert_stderr_contains_all(
            &stderr,
            &[
                "cem.run_config.unsupported_schema_identity",
                "https://cem.dev/ns/cli/run-config/1",
            ],
        );
        assert!(
            !stderr.contains("I/O error"),
            "config schema diagnostics must fail before input files are read: {stderr}"
        );
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            report["diagnostics"][0]["code"],
            "cem.run_config.unsupported_schema_identity"
        );
    }

    #[test]
    fn parse_config_diagnostics_write_requested_report_before_document_parsing() {
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/parse-bad-config.json");
        let report_path =
            std::env::temp_dir().join("cem-ml-cli-tests/parse-bad-config-report.json");
        let _ = std::fs::remove_file(&report_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{ "uri": "/definitely/not/read.cem" }],
                "outputs": [{ "inputRef": "missing.cem" }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "parse",
                "--config",
                config_path.to_str().unwrap(),
                "--report-json",
                report_path.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("cem.run_config.output_input_ref_unknown"));
        assert!(
            !stderr.contains("I/O error"),
            "config diagnostics must fail before input files are read: {stderr}"
        );
        let stdout_report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            stdout_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
        let file_report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        assert_eq!(
            file_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
    }

    #[test]
    fn convert_config_diagnostics_write_requested_report_before_document_parsing() {
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/convert-bad-config.json");
        let report_path =
            std::env::temp_dir().join("cem-ml-cli-tests/convert-bad-config-report.json");
        let _ = std::fs::remove_file(&report_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{ "uri": "/definitely/not/read.cem" }],
                "outputs": [{ "inputRef": "missing.cem" }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--config",
                config_path.to_str().unwrap(),
                "--report-json",
                report_path.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("cem.run_config.output_input_ref_unknown"));
        assert!(
            !stderr.contains("I/O error"),
            "config diagnostics must fail before input files are read: {stderr}"
        );
        let stdout_report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            stdout_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
        let file_report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        assert_eq!(
            file_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
    }

    #[test]
    fn bench_config_diagnostics_write_requested_report_before_document_parsing() {
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/bench-bad-config.json");
        let report_path =
            std::env::temp_dir().join("cem-ml-cli-tests/bench-bad-config-report.json");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        let _ = std::fs::remove_file(&report_path);
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{ "uri": "/definitely/not/read.cem" }],
                "outputs": [{ "inputRef": "missing.cem" }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "bench",
                "--config",
                config_path.to_str().unwrap(),
                "--report-json",
                report_path.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("cem.run_config.output_input_ref_unknown"));
        assert!(
            !stderr.contains("I/O error"),
            "config diagnostics must fail before input files are read: {stderr}"
        );
        let stdout_report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            stdout_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
        let file_report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
        assert_eq!(
            file_report["diagnostics"][0]["code"],
            "cem.run_config.output_input_ref_unknown"
        );
    }

    #[test]
    fn remote_uri_config_path_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--config", "https://example.test/run.json"],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, "https://example.test/run.json");
    }

    #[test]
    fn localhost_file_uri_config_path_is_read() {
        let input = write_fixture("validate-localhost-file-uri-config-input.cem", "{p Hi}");
        let config_path =
            std::env::temp_dir().join("cem-ml-cli-tests/validate-localhost-file-uri-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{ "uri": input.display().to_string() }]
            })
            .to_string(),
        )
        .unwrap();
        let config_uri = localhost_file_uri(&config_path);

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--config", &config_uri],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["inputCount"], 1);
        assert_eq!(v["inputs"][0], input.display().to_string());
    }

    #[test]
    fn non_local_file_uri_config_path_is_rejected() {
        let uri = "file://example.test/run.json";
        let (outcome, stdout, stderr) =
            run(&RealCemMlEngine::new(), &["validate", "--config", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn custom_uri_config_path_is_rejected_without_resolver() {
        let uri = "cem+vfs://workspace/run.json";
        let (outcome, stdout, stderr) =
            run(&RealCemMlEngine::new(), &["validate", "--config", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn namespace_context_diagnostics_fail_before_document_parsing() {
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--namespace",
                "xml=urn:not-xml",
                "/definitely/not/read.cem",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("cem.run_config.scope_namespace_invalid"));
        assert!(
            !stderr.contains("I/O error"),
            "namespace diagnostics must fail before positional inputs are read: {stderr}"
        );
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = report["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.run_config.scope_namespace_invalid"));
    }

    #[test]
    fn config_relative_module_map_is_resolved_against_config_path() {
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/config-module-map");
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.cem");
        let config_path = dir.join("run.json");
        std::fs::write(
            &input,
            r#"@schema src="ui/button"
{p Hi}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("cem.modules.json"),
            r#"{"schemas":{"ui/button":"./schemas/button.schema"}}"#,
        )
        .unwrap();
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string(),
                    "rootScope": {
                        "moduleMap": "cem.modules.json"
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_unreadable"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.module_map_invalid"));
    }

    #[test]
    fn root_scope_config_diagnostics_fail_before_document_parsing() {
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/bad-root-scope.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": "/definitely/not/read.cem",
                    "rootScope": {
                        "namespaces": { "xml": "urn:not-xml" },
                        "moduleMap": ""
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["validate", "--config", config_path.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("cem.run_config.scope_namespace_invalid"));
        assert!(
            !stderr.contains("I/O error"),
            "root-scope diagnostics must fail before input files are read: {stderr}"
        );
        let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = report["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.run_config.scope_namespace_invalid"));
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.run_config.scope_module_map_invalid"));
    }

    #[test]
    fn validate_without_positional_or_spec_is_usage_error() {
        let (outcome, _, stderr) = run(&FakeEngine, &["validate"]);
        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("validate requires"));
    }

    #[test]
    fn validate_strict_fail_level_exits_one_when_any_diag_present() {
        let p = write_fixture("validate-strict.cem", "{x}");
        let (outcome, _, _) = run(
            &FakeEngine,
            &["validate", "--fail-level", "strict", p.to_str().unwrap()],
        );
        // FakeEngine emits one info diagnostic per input → strict treats it as failure.
        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE);
    }

    #[test]
    fn check_with_zero_hard_violations_succeeds_when_only_info() {
        let p = write_fixture("check-zhv.cem", "{x}");
        let (outcome, _, _) = run(
            &FakeEngine,
            &["check", "--zero-hard-violations", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
    }

    #[test]
    fn bench_remote_uri_input_is_rejected_without_resolver() {
        let uri = "https://example.test/bench.cem";
        assert_remote_input_uri_rejected(&["bench", "--format", "json", uri], uri);
    }

    #[test]
    fn bench_file_uri_input_reads_local_input() {
        let input = write_fixture("bench-file-uri-input.cem", "{x}");
        let file_uri = local_file_uri(&input);
        let (outcome, stdout, stderr) = run(&FakeEngine, &["bench", "--format", "json", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["inputs"][0], file_uri);
    }

    #[test]
    fn validate_writes_report_files_when_requested() {
        let p = write_fixture("validate-rep.cem", "{x}");
        let json_path = std::env::temp_dir().join("cem-ml-cli-tests/v.report.json");
        let md_path = std::env::temp_dir().join("cem-ml-cli-tests/v.report.md");
        let _ = std::fs::remove_file(&json_path);
        let _ = std::fs::remove_file(&md_path);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "validate",
                "--report-json",
                json_path.to_str().unwrap(),
                "--report-md",
                md_path.to_str().unwrap(),
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        let json = std::fs::read_to_string(&json_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["summary"]["inputCount"].as_u64().unwrap() >= 1);
        let md = std::fs::read_to_string(&md_path).unwrap();
        assert!(md.contains("cem-ml report"));
    }

    #[test]
    fn fixture_validate_uses_default_inputs_when_none_given() {
        let (outcome, stdout, _) = run(&FakeEngine, &["fixture", "validate"]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let count = v["summary"]["inputCount"].as_u64().unwrap();
        assert_eq!(count, (fixture_manifest_pairs().len() * 2) as u64);
    }

    #[test]
    fn fixture_inputs_infer_format_from_extension() {
        let inputs = collect_fixture_inputs(
            &[
                PathBuf::from("examples/cem-ml/login.cem"),
                PathBuf::from("examples/semantic/login.html"),
                PathBuf::from("examples/cem-ml/namespace-rebinding/default-html-svg-html.xml"),
            ],
            false,
            &ScopeConfig::default(),
        );
        assert_eq!(inputs[0].from_format, Some(eng::InputFormat::Cem));
        assert_eq!(inputs[1].from_format, Some(eng::InputFormat::Html));
        assert_eq!(inputs[2].from_format, Some(eng::InputFormat::Xml));
    }

    #[test]
    fn fixture_validate_merges_template_embedding_diagnostics() {
        let p = write_fixture("fixture-broken-template.cem", "{p | {$ 1 + }}");
        let (outcome, stdout, _) = run(
            &FakeEngine,
            &[
                "fixture",
                "validate",
                "--zero-hard-violations",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert!(
            v["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"].as_str().unwrap_or("").starts_with("cem.ql.")),
            "fixture validate must surface cem-ql template diagnostics"
        );
        assert!(v["summary"]["hardViolationCount"].as_u64().unwrap() > 0);
    }

    #[test]
    fn fixture_validate_positional_file_uri_reads_local_input() {
        let p = write_fixture("fixture-validate-file-uri.cem", "{p Hi}");
        let file_uri = format!("file://{}", p.display());
        let (outcome, stdout, stderr) =
            run(&RealCemMlEngine::new(), &["fixture", "validate", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["summary"]["hardViolationCount"], 0);
        assert_eq!(v["summary"]["inputCount"], 1);
        assert_eq!(v["inputs"][0], file_uri);
    }

    #[test]
    fn fixture_validate_remote_uri_input_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["fixture", "validate", "https://example.test/fixture.cem"],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_input_resolver_boundary(&stderr, "https://example.test/fixture.cem");
    }

    #[test]
    fn fixture_validate_custom_uri_input_is_rejected_without_resolver() {
        let uri = "cem+vfs://workspace/fixture.cem";
        let (outcome, stdout, stderr) = run(&RealCemMlEngine::new(), &["fixture", "validate", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_input_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_validate_non_local_file_uri_input_is_rejected() {
        let uri = "file://example.test/fixture.cem";
        let (outcome, stdout, stderr) = run(&RealCemMlEngine::new(), &["fixture", "validate", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_validate_input_spec_fixture_validate_ms_budget_is_enforced() {
        let p = write_fixture("fixture-validate-budget.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "fixture",
                "validate",
                "--input-spec",
                &format!("uri={},budgets=fixtureValidateMs:0", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn fixture_roundtrip_input_spec_fixture_roundtrip_ms_budget_is_reported() {
        let p = write_fixture("fixture-roundtrip-budget.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "fixture",
                "roundtrip",
                "--input-spec",
                &format!("uri={},budgets=fixtureRoundtripMs:0", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["report"]["summary"]["inputCount"], 1);
        let diagnostics = v["report"]["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn fixture_roundtrip_positional_file_uri_reads_local_input() {
        let p = write_fixture("fixture-roundtrip-file-uri-input.cem", "{p Hi}");
        let file_uri = local_file_uri(&p);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["fixture", "roundtrip", &file_uri],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["report"]["summary"]["hardViolationCount"], 0);
        assert_eq!(v["report"]["summary"]["inputCount"], 1);
        assert_eq!(v["report"]["inputs"][0], file_uri);
        assert_eq!(v["artifacts"][0]["input"], file_uri);
    }

    #[test]
    fn fixture_roundtrip_remote_uri_input_is_rejected_without_resolver() {
        let uri = "https://example.test/fixture-roundtrip.cem";
        let (outcome, stdout, stderr) =
            run(&RealCemMlEngine::new(), &["fixture", "roundtrip", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_input_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_roundtrip_custom_uri_input_is_rejected_without_resolver() {
        let uri = "cem+vfs://workspace/fixture-roundtrip.cem";
        let (outcome, stdout, stderr) =
            run(&RealCemMlEngine::new(), &["fixture", "roundtrip", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_remote_input_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_roundtrip_non_local_file_uri_input_is_rejected() {
        let uri = "file://example.test/fixture-roundtrip.cem";
        let (outcome, stdout, stderr) =
            run(&RealCemMlEngine::new(), &["fixture", "roundtrip", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_manifest_pairs_every_top_level_cem_fixture_with_html_parity() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let pairs = fixture_manifest_pairs();
        assert!(!pairs.is_empty(), "fixture manifest must not be empty");

        let manifest_cem: std::collections::BTreeSet<String> =
            pairs.iter().map(|p| p.cem.clone()).collect();
        let disk_cem: std::collections::BTreeSet<String> =
            std::fs::read_dir(workspace.join("examples/cem-ml"))
                .unwrap()
                .filter_map(|entry| {
                    let path = entry.unwrap().path();
                    (path.extension().and_then(|e| e.to_str()) == Some("cem")).then(|| {
                        format!(
                            "examples/cem-ml/{}",
                            path.file_name().unwrap().to_string_lossy()
                        )
                    })
                })
                .collect();
        assert_eq!(manifest_cem, disk_cem);

        for pair in pairs {
            assert!(workspace.join(&pair.cem).is_file(), "missing {}", pair.cem);
            assert!(
                workspace.join(&pair.html).is_file(),
                "missing {}",
                pair.html
            );
        }
    }

    #[test]
    fn bench_emits_json_when_requested() {
        let p = write_fixture("bench.cem", "{x}");
        let (outcome, stdout, _) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--iterations",
                "3",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "fake-bench");
        assert_eq!(v["iterations"], 3);
    }

    #[test]
    fn bench_input_spec_bench_ms_budget_is_enforced() {
        let p = write_fixture("bench-budget.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "bench",
                "--iterations",
                "1",
                "--input-spec",
                &format!("uri={},budgets=benchMs:0", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["budgetExceeded"], true);
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn scope_budget_context_option_is_enforced_for_positional_input() {
        let p = write_fixture("validate-context-scope-budget.cem", r#"{p Hi}"#);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "validate",
                "--format",
                "json",
                "--scope-budget",
                "validateMs=0",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_HARD_FAILURE, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn inspect_routes_view_through_engine() {
        let p = write_fixture("inspect.cem", "{x}");
        let (outcome, stdout, _) = run(
            &FakeEngine,
            &["inspect", "--show", "events", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["view"], "events");
    }

    #[test]
    fn inspect_remote_uri_out_destination_is_rejected_without_resolver() {
        let p = write_fixture("inspect-remote-out.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "inspect",
                "--out",
                "https://example.test/inspect.json",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/inspect.json");
    }

    #[test]
    fn inspect_remote_uri_input_is_rejected_without_resolver() {
        let uri = "https://example.test/inspect.cem";
        assert_remote_input_uri_rejected(&["inspect", uri], uri);
    }

    #[test]
    fn inspect_file_uri_input_reads_local_input() {
        let input = write_fixture("inspect-file-uri-input.cem", "{x}");
        let file_uri = local_file_uri(&input);
        let (outcome, stdout, stderr) = run(&FakeEngine, &["inspect", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["input"], file_uri);
    }

    #[test]
    fn inspect_file_uri_out_destination_writes_local_output() {
        let p = write_fixture("inspect-file-uri-out.cem", "{x}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/inspect-file-uri-out.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "inspect",
                "--out",
                &format!("file://{}", out_path.display()),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-inspect");
    }

    #[test]
    fn inspect_input_spec_inspect_ms_budget_is_reported() {
        let p = write_fixture("inspect-budget.cem", "{p Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "inspect",
                "--show",
                "diagnostics",
                "--input-spec",
                &format!("uri={},budgets=inspectMs:0", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn trace_uses_run_config_scheduler() {
        let input = write_fixture("trace-scheduler.cem", "{p Hi}");
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/trace-scheduler.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string(),
                    "rootScope": {
                        "budgets": {
                            "queueSize": "12",
                            "pluginTimeBudgetMs": "7"
                        }
                    }
                }],
                "scheduler": {
                    "threadPool": "deterministic",
                    "maxParallelDocuments": 3
                }
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["trace", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["scheduler"]["threadPool"], "deterministic");
        assert_eq!(v["scheduler"]["maxParallelDocuments"], 3);
        assert_eq!(v["scheduler"]["policy"]["cpuWorkers"], 3);
        assert_eq!(v["scheduler"]["policy"]["queueSize"], 12);
        assert_eq!(v["scheduler"]["policy"]["pluginTimeBudgetMs"], 7);
    }

    #[test]
    fn trace_remote_uri_out_destination_is_rejected_without_resolver() {
        let p = write_fixture("trace-remote-out.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "trace",
                "--out",
                "https://example.test/trace.json",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/trace.json");
    }

    #[test]
    fn trace_remote_uri_input_is_rejected_without_resolver() {
        let uri = "https://example.test/trace.cem";
        assert_remote_input_uri_rejected(&["trace", uri], uri);
    }

    #[test]
    fn trace_file_uri_input_reads_local_input() {
        let input = write_fixture("trace-file-uri-input.cem", "{x}");
        let file_uri = local_file_uri(&input);
        let (outcome, stdout, stderr) = run(&FakeEngine, &["trace", &file_uri]);

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["input"], file_uri);
    }

    #[test]
    fn trace_file_uri_out_destination_writes_local_output() {
        let p = write_fixture("trace-file-uri-out.cem", "{x}");
        let out_path = std::env::temp_dir().join("cem-ml-cli-tests/trace-file-uri-out.json");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "trace",
                "--out",
                &format!("file://{}", out_path.display()),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.trim().is_empty());
        let written = std::fs::read_to_string(&out_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["kind"], "fake-trace");
    }

    #[test]
    fn trace_config_trace_ms_budget_is_reported() {
        let input = write_fixture("trace-budget.cem", "{p Hi}");
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/trace-budget.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": input.display().to_string(),
                    "rootScope": {
                        "budgets": {
                            "traceMs": "0"
                        }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap();

        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["trace", "--config", config_path.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["report"]["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn convert_passes_target_identity_to_engine() {
        let p = write_fixture("convert-target.html", "<p>Hi</p>");
        let (outcome, stdout, _) = run(
            &FakeEngine,
            &[
                "convert",
                "--content-type",
                "text/html",
                "--to-content-type",
                "application/cem+xml",
                "--to-schema",
                "https://cem.dev/ns/core/1",
                "--base-uri",
                "file:///tmp/",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["toFormat"], "dom-json");
        assert_eq!(v["target"]["contentType"], "application/cem+xml");
        assert_eq!(v["target"]["schema"], "https://cem.dev/ns/core/1");
        assert_eq!(v["target"]["baseUri"], "file:///tmp/");
    }

    #[test]
    fn convert_to_format_html_renders_light_dom_html() {
        let p = write_fixture("convert-html.cem", "@doc cem-ml 1\n{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--to-format", "html", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn convert_to_content_type_html_selects_html_export_adapter() {
        let p = write_fixture("convert-target-html.cem", "@doc cem-ml 1\n{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-content-type",
                "text/html",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn convert_to_content_type_xhtml_selects_html_export_adapter() {
        let p = write_fixture("convert-target-xhtml.cem", "@doc cem-ml 1\n{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-content-type",
                "application/xhtml+xml",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn convert_to_content_type_xml_selects_xml_export_adapter() {
        let p = write_fixture("convert-target-xml.cem", "@doc cem-ml 1\n{p @id=one | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-content-type",
                "application/xml",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "xml");
        assert_eq!(v["content"], r#"<p id="one">Hi</p>"#);
    }

    #[test]
    fn convert_to_content_type_svg_selects_xml_export_adapter() {
        let p = write_fixture("convert-target-svg.cem", "{svg | {title | Hi}}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "cem",
                "--to-content-type",
                "image/svg+xml",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "xml");
        assert_eq!(v["content"], "<svg><title>Hi</title></svg>");
    }

    #[test]
    fn convert_to_format_xml_renders_xml() {
        let p = write_fixture("convert-format-xml.cem", "{input @required}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--to-format", "xml", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "xml");
        assert_eq!(v["content"], r#"<input required=""/>"#);
    }

    #[test]
    fn convert_to_schema_cem_core_selects_cem_export_adapter() {
        let p = write_fixture("convert-target-schema.cem", "@doc cem-ml 1\n{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-schema",
                "https://cem.dev/ns/core/1",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "cem");
        assert_eq!(v["content"], "@doc cem-ml 1\n{p | Hi}\n");
    }

    #[test]
    fn convert_to_html_schema_selects_html_export_adapter() {
        let p = write_fixture("convert-target-html-schema.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--to-schema",
                "http://www.w3.org/1999/xhtml",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn convert_to_svg_schema_selects_html_export_adapter() {
        let p = write_fixture("convert-target-svg-schema.cem", "{svg | {title | Hi}}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--to-schema",
                "http://www.w3.org/2000/svg",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<svg><title>Hi</title></svg>");
    }

    #[test]
    fn convert_to_transform_config_schema_selects_cem_export_adapter() {
        let p = write_fixture("convert-target-transform-config-schema.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--to-schema",
                cem_ml::transform_config::TRANSFORM_CONFIG_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "cem");
        assert_eq!(v["content"], "{p | Hi}\n");
    }

    #[test]
    fn convert_to_native_template_schema_selects_cem_export_adapter() {
        let p = write_fixture("convert-target-native-template-schema.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--to-schema",
                cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "cem");
        assert_eq!(v["content"], "{p | Hi}\n");
    }

    #[test]
    fn convert_to_default_namespace_cem_core_selects_cem_export_adapter() {
        let p = write_fixture(
            "convert-target-default-namespace.cem",
            "@doc cem-ml 1\n{p | Hi}",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--default-namespace",
                "https://cem.dev/ns/core/1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "cem");
        assert_eq!(v["content"], "@doc cem-ml 1\n{p | Hi}\n");
    }

    #[test]
    fn convert_to_default_namespace_html_selects_html_export_adapter() {
        let p = write_fixture(
            "convert-target-html-default-namespace.cem",
            "@doc cem-ml 1\n{p | Hi}",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--default-namespace",
                "http://www.w3.org/1999/xhtml",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "html");
        assert_eq!(v["content"], "<p>Hi</p>");
    }

    #[test]
    fn convert_to_dom_json_projection_schema_selects_dom_json_export_adapter() {
        let p = write_fixture("convert-target-dom-json-schema.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "xml",
                "--to-content-type",
                "application/json",
                "--to-schema",
                cem_ml::lifecycle::DOM_JSON_PROJECTION_SCHEMA,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "document");
        assert_eq!(v["children"][0]["kind"], "element");
        assert_eq!(v["children"][0]["name"], "p");
    }

    #[test]
    fn convert_to_dom_binary_projection_content_type_selects_binary_export_adapter() {
        let p = write_fixture("convert-target-dom-bin.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-content-type",
                cem_ml::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        assert!(stdout.as_bytes().starts_with(b"CEMPROJ\0"));
        assert!(!stdout.trim_start().starts_with('{'));
    }

    #[test]
    fn convert_to_format_dom_bin_outputs_raw_binary_projection() {
        let p = write_fixture("convert-format-dom-bin.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &["convert", "--to-format", "dom-bin", p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.as_bytes().starts_with(b"CEMPROJ\0"));
        assert!(!stdout.trim_start().starts_with('{'));
    }

    #[test]
    fn convert_to_format_dom_bin_out_writes_raw_binary_file() {
        let p = write_fixture("convert-format-dom-bin-out.cem", "{p | Hi}");
        let out = std::env::temp_dir().join("cem-ml-cli-tests/convert-format-dom-bin-out.cembin");
        let _ = std::fs::remove_file(&out);
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "dom-bin",
                "--out",
                out.to_str().unwrap(),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(stdout.is_empty());
        let bytes = std::fs::read(out).unwrap();
        assert!(bytes.starts_with(b"CEMPROJ\0"));
    }

    #[test]
    fn convert_to_ast_projection_schema_selects_ast_export_adapter() {
        let p = write_fixture("convert-target-ast-schema.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "cem",
                "--to-schema",
                cem_ml::lifecycle::AST_PROJECTION_SCHEMA,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["kind"], "document");
        assert_eq!(v["children"][0]["name"], "p");
    }

    #[test]
    fn convert_to_events_projection_schema_selects_events_export_adapter() {
        let p = write_fixture("convert-target-events-schema.cem", "{p | Hi}");
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "html",
                "--to-content-type",
                "application/json",
                "--to-schema",
                cem_ml::lifecycle::EVENTS_PROJECTION_SCHEMA,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            !stderr.contains("cem.lifecycle.target_adapter_unsupported"),
            "{stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert!(v.is_array());
        assert!(v
            .as_array()
            .unwrap()
            .iter()
            .any(|event| { event["kind"] == "open" && event["name"] == "p" }));
    }

    #[test]
    fn convert_to_unknown_schema_reports_unsupported_target_adapter() {
        let p = write_fixture(
            "convert-target-unknown-schema.cem",
            "@doc cem-ml 1\n{p | Hi}",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--to-schema",
                "https://example.test/ns/widgets/1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert!(v.is_array());
        assert!(stderr.contains("cem.lifecycle.target_adapter_unsupported"));
        assert!(stderr.contains("target schema `https://example.test/ns/widgets/1`"));
    }

    #[test]
    fn convert_to_unknown_namespace_reports_unsupported_target_adapter() {
        let p = write_fixture(
            "convert-target-unknown-namespace.cem",
            "@doc cem-ml 1\n{p | Hi}",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--namespace",
                "widget=https://example.test/ns/widgets/1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert!(v.is_array());
        assert!(stderr.contains("cem.lifecycle.target_adapter_unsupported"));
        assert!(stderr.contains("target namespace `widget=https://example.test/ns/widgets/1`"));
    }

    #[test]
    fn convert_to_unknown_content_type_and_schema_reports_full_target_identity() {
        let p = write_fixture(
            "convert-target-unknown-content-type-and-schema.cem",
            "@doc cem-ml 1\n{p | Hi}",
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--to-format",
                "events",
                "--to-content-type",
                "application/unknown",
                "--to-schema",
                "https://example.test/ns/widgets/1",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert!(v.is_array());
        assert!(stderr.contains("cem.lifecycle.target_adapter_unsupported"));
        assert!(stderr.contains("target content type `application/unknown`"));
        assert!(stderr.contains("schema `https://example.test/ns/widgets/1`"));
    }

    #[test]
    fn convert_legacy_custom_element_content_type_routes_to_engine_lowering() {
        let p = write_fixture(
            "legacy-custom-element.html",
            r#"<if test="not($disabled)"><button>Go</button></if>"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--content-type",
                cem_ml::legacy_custom_element::TEMPLATE_LANG,
                "--to-content-type",
                "application/cem+xml",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            v["content"].as_str().unwrap(),
            "{cem:if @test=\"not (disabled)\" | {button | Go}}\n"
        );
    }

    #[test]
    fn convert_xslt_namespace_routes_to_engine_lowering() {
        let p = write_fixture(
            "legacy-custom-element-xsl-namespace.data",
            r#"<xsl:if test="$ready"><button>Go</button></xsl:if>"#,
        );
        let (outcome, stdout, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "convert",
                "--namespace",
                "xsl=http://www.w3.org/1999/XSL/Transform",
                "--to-content-type",
                "application/cem+xml",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(
            v["content"].as_str().unwrap(),
            "{cem:if @test=\"ready\" | {button | Go}}\n"
        );
    }

    #[test]
    fn fixture_validate_with_dir_uses_default_basename() {
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/fv-dir");
        let _ = std::fs::remove_dir_all(&dir);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "fixture",
                "validate",
                "--report-json",
                dir.to_str().unwrap(),
                "--report-md",
                dir.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(dir.join("cem-ml.report.json").is_file());
        assert!(dir.join("cem-ml.report.md").is_file());
    }

    #[test]
    fn fixture_roundtrip_with_dir_uses_roundtrip_basename() {
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/fr-dir");
        let _ = std::fs::remove_dir_all(&dir);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "fixture",
                "roundtrip",
                "--report-json",
                dir.to_str().unwrap(),
                "--report-md",
                dir.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(
            dir.join("cem-ml.roundtrip.report.json").is_file(),
            "missing roundtrip.report.json"
        );
        assert!(
            dir.join("cem-ml.roundtrip.report.md").is_file(),
            "missing roundtrip.report.md"
        );
        assert!(
            !dir.join("cem-ml.report.json").exists(),
            "should not have written validate basename"
        );
    }

    #[test]
    fn fixture_roundtrip_file_uri_dir_uses_roundtrip_basename() {
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/fr-file-uri-dir");
        let _ = std::fs::remove_dir_all(&dir);
        let (outcome, _, stderr) = run(
            &FakeEngine,
            &[
                "fixture",
                "roundtrip",
                "--report-json",
                &format!("file://{}", dir.display()),
                "--report-md",
                &format!("file://{}", dir.display()),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            dir.join("cem-ml.roundtrip.report.json").is_file(),
            "missing roundtrip.report.json"
        );
        assert!(
            dir.join("cem-ml.roundtrip.report.md").is_file(),
            "missing roundtrip.report.md"
        );
    }

    #[test]
    fn fixture_roundtrip_remote_uri_report_destination_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "fixture",
                "roundtrip",
                "--report-json",
                "https://example.test/fixture-roundtrip-report.json",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(
            &stderr,
            "https://example.test/fixture-roundtrip-report.json",
        );
    }

    #[test]
    fn fixture_roundtrip_remote_uri_markdown_report_destination_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "fixture",
                "roundtrip",
                "--report-md",
                "https://example.test/fixture-roundtrip-report.md",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(
            &stderr,
            "https://example.test/fixture-roundtrip-report.md",
        );
    }

    #[test]
    fn fixture_roundtrip_custom_uri_report_destination_is_rejected_without_resolver() {
        let uri = "cem+vfs://workspace/fixture-roundtrip-report.json";
        let (outcome, stdout, stderr) =
            run(&FakeEngine, &["fixture", "roundtrip", "--report-json", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_roundtrip_custom_uri_markdown_report_destination_is_rejected_without_resolver() {
        let uri = "cem+vfs://workspace/fixture-roundtrip-report.md";
        let (outcome, stdout, stderr) =
            run(&FakeEngine, &["fixture", "roundtrip", "--report-md", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_roundtrip_non_local_file_uri_markdown_report_destination_is_rejected() {
        let uri = "file://example.test/fixture-roundtrip-report.md";
        let (outcome, stdout, stderr) =
            run(&FakeEngine, &["fixture", "roundtrip", "--report-md", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_validate_remote_uri_report_destination_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "fixture",
                "validate",
                "--report-json",
                "https://example.test/fixture-report.json",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/fixture-report.json");
    }

    #[test]
    fn fixture_validate_remote_uri_markdown_report_destination_is_rejected_without_resolver() {
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "fixture",
                "validate",
                "--report-md",
                "https://example.test/fixture-report.md",
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/fixture-report.md");
    }

    #[test]
    fn fixture_validate_custom_uri_report_destination_is_rejected_without_resolver() {
        let uri = "cem+vfs://workspace/fixture-report.json";
        let (outcome, stdout, stderr) =
            run(&FakeEngine, &["fixture", "validate", "--report-json", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_validate_custom_uri_markdown_report_destination_is_rejected_without_resolver() {
        let uri = "cem+vfs://workspace/fixture-report.md";
        let (outcome, stdout, stderr) =
            run(&FakeEngine, &["fixture", "validate", "--report-md", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn fixture_validate_non_local_file_uri_markdown_report_destination_is_rejected() {
        let uri = "file://example.test/fixture-report.md";
        let (outcome, stdout, stderr) =
            run(&FakeEngine, &["fixture", "validate", "--report-md", uri]);

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn bench_with_dir_uses_bench_basename() {
        let p = write_fixture("bench-dir.cem", "{x}");
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/bench-dir");
        let _ = std::fs::remove_dir_all(&dir);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--report-json",
                dir.to_str().unwrap(),
                "--report-md",
                dir.to_str().unwrap(),
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(
            dir.join("cem-ml.bench.report.json").is_file(),
            "missing bench.report.json"
        );
        assert!(
            dir.join("cem-ml.bench.report.md").is_file(),
            "missing bench.report.md"
        );
        let markdown = std::fs::read_to_string(dir.join("cem-ml.bench.report.md")).unwrap();
        assert!(markdown.contains("# cem-ml benchmark report"));
        assert!(markdown.contains("\"kind\": \"fake-bench\""));
    }

    #[test]
    fn bench_with_file_uri_dir_uses_bench_basename() {
        let p = write_fixture("bench-file-uri-dir.cem", "{x}");
        let dir = std::env::temp_dir().join("cem-ml-cli-tests/bench-file-uri-dir");
        let _ = std::fs::remove_dir_all(&dir);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--report-json",
                &format!("file://{}", dir.display()),
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(
            dir.join("cem-ml.bench.report.json").is_file(),
            "missing bench.report.json"
        );
    }

    #[test]
    fn report_explicit_file_path_overrides_basename() {
        let p = write_fixture("validate-explicit.cem", "{x}");
        let json_path = std::env::temp_dir().join("cem-ml-cli-tests/custom-name.json");
        let _ = std::fs::remove_file(&json_path);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "validate",
                "--report-json",
                json_path.to_str().unwrap(),
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(json_path.is_file(), "explicit filename should be honored");
    }

    #[test]
    fn report_explicit_file_uri_path_writes_local_report() {
        let p = write_fixture("validate-explicit-file-uri.cem", "{x}");
        let json_path = std::env::temp_dir().join("cem-ml-cli-tests/custom-file-uri-name.json");
        let _ = std::fs::remove_file(&json_path);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "validate",
                "--report-json",
                &format!("file://{}", json_path.display()),
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(
            json_path.is_file(),
            "file URI report path should be honored"
        );
    }

    #[test]
    fn report_explicit_localhost_file_uri_path_writes_local_report() {
        let p = write_fixture("validate-explicit-localhost-file-uri.cem", "{x}");
        let json_path =
            std::env::temp_dir().join("cem-ml-cli-tests/custom-localhost-file-uri-name.json");
        let _ = std::fs::remove_file(&json_path);
        let (outcome, _, stderr) = run(
            &FakeEngine,
            &[
                "validate",
                "--report-json",
                &localhost_file_uri(&json_path),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(
            json_path.is_file(),
            "localhost file URI report path should be honored"
        );
    }

    #[test]
    fn markdown_report_explicit_file_uri_path_writes_local_report() {
        let p = write_fixture("validate-explicit-md-file-uri.cem", "{x}");
        let md_path = std::env::temp_dir().join("cem-ml-cli-tests/custom-file-uri-name.md");
        let _ = std::fs::remove_file(&md_path);
        let (outcome, _, stderr) = run(
            &FakeEngine,
            &[
                "validate",
                "--report-md",
                &format!("file://{}", md_path.display()),
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let markdown = std::fs::read_to_string(&md_path).unwrap();
        assert!(markdown.contains("# cem-ml report"));
        assert!(markdown.contains("- inputs: 1"));
    }

    #[test]
    fn validate_remote_uri_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("validate-remote-report-destination.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "validate",
                "--report-json",
                "https://example.test/report.json",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/report.json");
    }

    #[test]
    fn validate_non_local_file_uri_report_destination_is_rejected() {
        let p = write_fixture("validate-non-local-file-uri-report-destination.cem", "{x}");
        let uri = "file://example.test/report.json";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["validate", "--report-json", uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn validate_custom_uri_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("validate-custom-uri-report-destination.cem", "{x}");
        let uri = "cem+vfs://workspace/report.json";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["validate", "--report-json", uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn validate_remote_uri_markdown_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("validate-remote-md-report-destination.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "validate",
                "--report-md",
                "https://example.test/report.md",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/report.md");
    }

    #[test]
    fn validate_custom_uri_markdown_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("validate-custom-uri-md-report-destination.cem", "{x}");
        let uri = "cem+vfs://workspace/report.md";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["validate", "--report-md", uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn validate_non_local_file_uri_markdown_report_destination_is_rejected() {
        let p = write_fixture(
            "validate-non-local-file-uri-md-report-destination.cem",
            "{x}",
        );
        let uri = "file://example.test/report.md";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["validate", "--report-md", uri, p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.trim().is_empty());
        assert_stderr_contains_all(&stderr, &["report write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn bench_remote_uri_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("bench-remote-report-destination.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--report-json",
                "https://example.test/bench.json",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-bench\""));
        assert_stderr_contains_all(&stderr, &["benchmark report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/bench.json");
    }

    #[test]
    fn bench_custom_uri_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("bench-custom-report-destination.cem", "{x}");
        let uri = "cem+vfs://workspace/bench.json";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--report-json",
                uri,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-bench\""));
        assert_stderr_contains_all(&stderr, &["benchmark report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn bench_remote_uri_markdown_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("bench-remote-md-report-destination.cem", "{x}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--report-md",
                "https://example.test/bench.md",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-bench\""));
        assert_stderr_contains_all(&stderr, &["benchmark report write failure"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/bench.md");
    }

    #[test]
    fn bench_custom_uri_markdown_report_destination_is_rejected_without_resolver() {
        let p = write_fixture("bench-custom-md-report-destination.cem", "{x}");
        let uri = "cem+vfs://workspace/bench.md";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--report-md",
                uri,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-bench\""));
        assert_stderr_contains_all(&stderr, &["benchmark report write failure"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn bench_non_local_file_uri_markdown_report_destination_is_rejected() {
        let p = write_fixture("bench-non-local-file-uri-md-report-destination.cem", "{x}");
        let uri = "file://example.test/bench.md";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "bench",
                "--format",
                "json",
                "--report-md",
                uri,
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.contains("\"kind\": \"fake-bench\""));
        assert_stderr_contains_all(&stderr, &["benchmark report write failure"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn quiet_suppresses_stdout_for_validate() {
        let p = write_fixture("validate-quiet.cem", "{x}");
        let (outcome, stdout, _) = run(&FakeEngine, &["--quiet", "validate", p.to_str().unwrap()]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.is_empty());
    }

    #[test]
    fn observe_events_flag_writes_jsonl_event_stream() {
        let p = write_fixture("observe-events.cem", "{p | hi}");
        let out_dir = std::env::temp_dir().join("cem-ml-cli-observe");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join("events.jsonl");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "--observe-events",
                out_path.to_str().unwrap(),
                "parse",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(out_path.is_file(), "observe-events should create the file");
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(!body.is_empty(), "event stream must not be empty");
        let mut channels = std::collections::HashSet::new();
        for line in body.lines() {
            let v: serde_json::Value = serde_json::from_str(line).expect("each line is JSON");
            channels.insert(v["channel"].as_str().unwrap().to_owned());
        }
        // Tier A parse always crosses tokenizer + normalizer + AST builder,
        // and emits at least one parse event for the `{p}` open.
        assert!(channels.contains("parse"));
        assert!(channels.contains("transform"));
    }

    #[test]
    fn observe_events_file_uri_writes_jsonl_event_stream() {
        let p = write_fixture("observe-events-file-uri.cem", "{p | hi}");
        let out_dir = std::env::temp_dir().join("cem-ml-cli-observe-file-uri");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join("events.jsonl");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, _, _) = run(
            &FakeEngine,
            &[
                "--observe-events",
                &format!("file://{}", out_path.display()),
                "parse",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(out_path.is_file(), "observe-events should create the file");
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(!body.is_empty(), "event stream must not be empty");
    }

    #[test]
    fn observe_events_localhost_file_uri_writes_jsonl_event_stream() {
        let p = write_fixture("observe-events-localhost-file-uri.cem", "{p | hi}");
        let out_dir = std::env::temp_dir().join("cem-ml-cli-observe-localhost-file-uri");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join("events.jsonl");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, _, stderr) = run(
            &FakeEngine,
            &[
                "--observe-events",
                &localhost_file_uri(&out_path),
                "parse",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        assert!(out_path.is_file(), "observe-events should create the file");
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(!body.is_empty(), "event stream must not be empty");
    }

    #[test]
    fn observe_events_remote_uri_destination_is_rejected_without_resolver() {
        let p = write_fixture("observe-events-remote-destination.cem", "{p | hi}");
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &[
                "--observe-events",
                "https://example.test/events.jsonl",
                "parse",
                p.to_str().unwrap(),
            ],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.is_empty());
        assert_stderr_contains_all(&stderr, &["--observe-events failed"]);
        assert_remote_resolver_boundary(&stderr, "https://example.test/events.jsonl");
    }

    #[test]
    fn observe_events_non_local_file_uri_destination_is_rejected() {
        let p = write_fixture(
            "observe-events-non-local-file-uri-destination.cem",
            "{p | hi}",
        );
        let uri = "file://example.test/events.jsonl";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["--observe-events", uri, "parse", p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.is_empty());
        assert_stderr_contains_all(&stderr, &["--observe-events failed"]);
        assert_local_file_uri_boundary(&stderr, uri);
    }

    #[test]
    fn observe_events_custom_uri_destination_is_rejected_without_resolver() {
        let p = write_fixture("observe-events-custom-uri-destination.cem", "{p | hi}");
        let uri = "cem+vfs://workspace/events.jsonl";
        let (outcome, stdout, stderr) = run(
            &FakeEngine,
            &["--observe-events", uri, "parse", p.to_str().unwrap()],
        );

        assert_eq!(outcome.exit_code, EXIT_IO);
        assert!(stdout.is_empty());
        assert_stderr_contains_all(&stderr, &["--observe-events failed"]);
        assert_remote_resolver_boundary(&stderr, uri);
    }

    #[test]
    fn observe_events_dash_writes_jsonl_to_stdout() {
        let p = write_fixture("observe-events-stdout.cem", "{p | hi}");
        let (outcome, stdout, _) = run(
            &FakeEngine,
            &["--observe-events", "-", "parse", p.to_str().unwrap()],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        // Stdout carries the JSONL events stream plus the normal
        // parse projection JSON. The first non-empty line should parse
        // as a JSONL event.
        let first = stdout.lines().next().expect("at least one output line");
        let v: serde_json::Value =
            serde_json::from_str(first).expect("first line of stdout is JSONL");
        assert!(v.get("channel").is_some(), "channel field must be present");
        assert!(
            v.get("sequence").is_some(),
            "sequence field must be present"
        );
    }

    #[test]
    fn observe_events_uses_input_spec_inputs() {
        let p = write_fixture("observe-events-input-spec.html", "<button>Go</button>");
        let out_dir = std::env::temp_dir().join("cem-ml-cli-observe");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join("input-spec-events.jsonl");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, _, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "--observe-events",
                out_path.to_str().unwrap(),
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri={},contentType=text/html", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(
            body.lines()
                .any(|line| line.contains(r#""channel":"parse""#)),
            "configured input spec should produce parse events: {body}"
        );
    }

    #[test]
    fn observe_events_input_spec_observe_ms_budget_is_reported() {
        let p = write_fixture("observe-events-budget.cem", "{p | hi}");
        let out_dir = std::env::temp_dir().join("cem-ml-cli-observe");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join("budget-events.jsonl");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, _, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "--observe-events",
                out_path.to_str().unwrap(),
                "validate",
                "--format",
                "json",
                "--input-spec",
                &format!("uri={},budgets=observeMs:0", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(
            body.lines().any(|line| {
                line.contains(r#""channel":"validate""#)
                    && line.contains(r#""code":"cem.scope.budget_exceeded""#)
            }),
            "observeMs budget should emit a validate event: {body}"
        );
        assert!(
            !body.contains("cem.scope.budget_unenforced"),
            "observeMs should be recognized as enforced: {body}"
        );
    }

    #[test]
    fn observe_events_uses_config_inputs() {
        let p = write_fixture("observe-events-config.html", "<button>Go</button>");
        let config_path = std::env::temp_dir().join("cem-ml-cli-tests/observe-events-config.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "inputs": [{
                    "uri": p.display().to_string(),
                    "rootScope": { "defaultContentType": "text/html" }
                }]
            })
            .to_string(),
        )
        .unwrap();
        let out_dir = std::env::temp_dir().join("cem-ml-cli-observe");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join("config-events.jsonl");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, _, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "--observe-events",
                out_path.to_str().unwrap(),
                "validate",
                "--format",
                "json",
                "--config",
                config_path.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(
            body.lines()
                .any(|line| line.contains(r#""channel":"parse""#)),
            "configured run input should produce parse events: {body}"
        );
    }

    #[test]
    fn observe_events_fixture_file_uri_input_is_read() {
        let p = write_fixture("observe-events-fixture-file-uri.cem", "{p | hi}");
        let out_dir = std::env::temp_dir().join("cem-ml-cli-observe");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join("fixture-file-uri-events.jsonl");
        let _ = std::fs::remove_file(&out_path);
        let (outcome, _, stderr) = run(
            &RealCemMlEngine::new(),
            &[
                "--observe-events",
                out_path.to_str().unwrap(),
                "fixture",
                "validate",
                &format!("file://{}", p.display()),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(
            body.lines()
                .any(|line| line.contains(r#""channel":"parse""#)),
            "fixture file URI input should produce parse events: {body}"
        );
    }
}

fn observable_inputs(
    command: &cli::Command,
) -> Result<(Vec<eng::EngineInput>, eng::EngineContext), CliRequestError> {
    match command {
        cli::Command::Parse(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let input = single_configured_input(
                &context,
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context))
        }
        cli::Command::Validate(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let inputs = collect_configured_inputs(
                &context,
                &a.inputs,
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((inputs, context))
        }
        cli::Command::Check(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let inputs = collect_configured_inputs(
                &context,
                &a.inputs,
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((inputs, context))
        }
        cli::Command::Inspect(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let input = single_configured_input(
                &context,
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context))
        }
        cli::Command::Convert(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let output_defaults = output_scope_defaults(a);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), output_defaults),
            )?;
            let context = context_with_config(&a.context, &config);
            let input = single_configured_input(
                &context,
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context))
        }
        cli::Command::Trace(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let input = single_configured_input(
                &context,
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context))
        }
        cli::Command::Bench(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let inputs =
                collect_configured_inputs(&context, &a.inputs, None, &config, &input_defaults)?;
            Ok((inputs, context))
        }
        cli::Command::Fixture(cli::FixtureCmd::Validate(a)) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let mut inputs =
                collect_fixture_inputs(&a.inputs, config.inputs.is_empty(), &input_defaults);
            for spec in &config.inputs {
                inputs.push(engine_input_from_spec(&context, spec, None)?);
            }
            Ok((inputs, context))
        }
        cli::Command::Fixture(cli::FixtureCmd::Roundtrip(a)) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config_for_context(
                &a.context,
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let context = context_with_config(&a.context, &config);
            let mut inputs =
                collect_fixture_inputs(&a.inputs, config.inputs.is_empty(), &input_defaults);
            for spec in &config.inputs {
                inputs.push(engine_input_from_spec(&context, spec, None)?);
            }
            Ok((inputs, context))
        }
        _ => Ok((Vec::new(), eng::EngineContext::default())),
    }
}

fn emit_observability_events(
    command: &cli::Command,
    target: &Path,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    let (inputs, context) = match observable_inputs(command) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    if inputs.is_empty() {
        return Ok(());
    }

    let mut all_events: Vec<cem_ml::observability::ReportEvent> = Vec::new();
    let registry = cem_ml::lifecycle::LifecycleRegistry::with_builtin_adapters();
    for input in inputs {
        let input = if input.bytes.is_empty() {
            match materialize_fixture_input(&context, &input) {
                Ok(input) => input,
                Err(EngineError::Io { source, .. }) => {
                    let _ = writeln!(
                        s.stderr,
                        "cem-ml: --observe-events: cannot read {}: {source}",
                        input.uri
                    );
                    continue;
                }
                Err(error) => {
                    let _ = writeln!(
                        s.stderr,
                        "cem-ml: --observe-events: cannot read {}: {error}",
                        input.uri
                    );
                    continue;
                }
            }
        } else {
            input
        };
        let loaded = registry.load(&input, &context);
        let observer = cem_ml::observability::BufferingObserver::new();
        let _ = cem_ml::real::observe_pipeline_scoped(
            &loaded.bytes,
            loaded.from_format,
            &input.root_scope,
            &observer,
        );
        all_events.extend(observer.drain());
    }

    let jsonl = cem_ml::observability::events_to_jsonl(&all_events);
    if target.as_os_str() == "-" {
        s.stdout.write_all(jsonl.as_bytes())?;
        s.stdout.flush()?;
    } else {
        write_destination(
            &context,
            target,
            "observability event destination",
            ResolvePurpose::ObserveEvents,
            jsonl.as_bytes(),
        )?;
    }
    Ok(())
}

pub fn dispatch<E: CemMlEngine + ?Sized>(
    engine: &E,
    parsed: cli::Cli,
    s: &mut Streams<'_>,
) -> Outcome {
    if let Some(observe_path) = parsed.observe_events.as_ref() {
        if let Err(e) = emit_observability_events(&parsed.command, observe_path, s) {
            let _ = writeln!(s.stderr, "cem-ml: --observe-events failed: {e}");
            return Outcome::code(EXIT_IO);
        }
    }
    match parsed.command {
        cli::Command::Parse(a) => run_parse(engine, a, s),
        cli::Command::Validate(a) => run_validate(engine, a, s),
        cli::Command::Check(a) => run_check(engine, a, s),
        cli::Command::Inspect(a) => run_inspect(engine, a, s),
        cli::Command::Convert(a) => run_convert(engine, a, s),
        cli::Command::Trace(a) => run_trace(engine, a, s),
        cli::Command::Bench(a) => run_bench(engine, a, s),
        cli::Command::Fixture(cli::FixtureCmd::Validate(a)) => run_fixture_validate(engine, a, s),
        cli::Command::Fixture(cli::FixtureCmd::Roundtrip(a)) => run_fixture_roundtrip(engine, a, s),
        cli::Command::Version => run_version(s),
        cli::Command::Transform(a) => run_transform(engine, a, s),
        cli::Command::Schema(cli::SchemaCmd::Emit) => run_reserved("schema emit", s),
        cli::Command::Schema(cli::SchemaCmd::Sample) => run_reserved("schema sample", s),
        cli::Command::Schema(cli::SchemaCmd::Replace) => run_reserved("schema replace", s),
        cli::Command::Plugin(cli::PluginCmd::List) => run_reserved("plugin list", s),
        cli::Command::Plugin(cli::PluginCmd::Inspect) => run_reserved("plugin inspect", s),
        cli::Command::Plugin(cli::PluginCmd::Run) => run_reserved("plugin run", s),
    }
}
