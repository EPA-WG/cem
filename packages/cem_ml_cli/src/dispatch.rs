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
    ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite,
    ResolverDiagnostic, ResolverRegistry, ResourceResolver,
};
use cem_ml::run_config::{
    self, InputSpec, OutputSpec, ResolverSpec, RunConfig, RunConfigDefaults, ScopeConfig,
};
use std::collections::{BTreeMap, BTreeSet};
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
        let Some(mapping) = self
            .mappings
            .iter()
            .find(|mapping| request.uri.starts_with(&mapping.uri_prefix))
        else {
            return Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: request.direction,
            });
        };
        let suffix = request
            .uri
            .strip_prefix(&mapping.uri_prefix)
            .unwrap_or_default()
            .trim_start_matches('/');
        local_mirror_path(&mapping.local_root, suffix).map_err(|message| ResolverDiagnostic::Io {
            uri: request.uri.clone(),
            message,
        })
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
    Ok(eng::TemplateInput {
        uri: path.display().to_string(),
        bytes: read.bytes,
        identity: root_scope.format_identity_option(),
        root_scope,
    })
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
            schema: None,
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
        scheduler: Default::default(),
        resolver_registry: Default::default(),
    };
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

fn transform_data_scope(args: &cli::TransformArgs) -> ScopeConfig {
    ScopeConfig {
        default_content_type: args
            .data_content_type
            .clone()
            .or_else(|| run_config::infer_content_type_from_path(&args.data.display().to_string())),
        schema: args.data_schema.clone(),
        ..ScopeConfig::default()
    }
}

fn transform_template_scope(args: &cli::TransformArgs) -> ScopeConfig {
    ScopeConfig {
        default_content_type: args.template_content_type.clone().or_else(|| {
            run_config::infer_content_type_from_path(&args.template.display().to_string())
        }),
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

// Reserved transform dispatch does not call this until runtime support is enabled.
#[allow(dead_code)]
fn transform_request_from_args(
    context: &eng::EngineContext,
    args: &cli::TransformArgs,
) -> Result<eng::TransformRequest, CliRequestError> {
    let data = engine_input(context, &args.data, None, transform_data_scope(args))
        .map_err(CliRequestError::Engine)?;
    let template = template_input(context, &args.template, transform_template_scope(args))
        .map_err(CliRequestError::Engine)?;
    let template_identity = template
        .identity
        .clone()
        .unwrap_or_else(|| template.root_scope.format_identity());
    let template_kind = eng::classify_transform_template_identity(&template_identity)
        .map_err(|error| CliRequestError::Usage(error.to_string()))?;
    let target_scope = transform_target_scope(args);
    Ok(eng::TransformRequest {
        data,
        template,
        template_kind,
        template_entrypoint: eng::TransformTemplateEntrypoint::implicit(),
        params: BTreeMap::new(),
        preserve_source_offsets: false,
        context: context.clone(),
        target: target_scope.format_identity_option(),
        target_scope,
        scheduler_scope_ids: eng::TransformSchedulerScopeIds::default(),
        execution_policy: eng::TransformExecutionPolicy::default(),
    })
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
                            write_primary(&engine_context, &resp.primary, args.out.as_deref(), s)
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
                if let Err(e) = write_primary(
                    &engine_context,
                    &resp.primary,
                    Some(destination.as_path()),
                    s,
                ) {
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

fn write_primary(
    context: &eng::EngineContext,
    primary: &serde_json::Value,
    out: Option<&Path>,
    s: &mut Streams<'_>,
) -> io::Result<()> {
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
            if let Err(e) = write_primary(&engine_context, &resp.primary, out.as_deref(), s) {
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
    fn reserved_transform_exits_two() {
        let (outcome, _, stderr) = run(
            &NotImplementedEngine,
            &[
                "transform",
                "data.xml",
                "--data-content-type",
                "application/xml",
                "--template",
                "view.xsl",
                "--template-content-type",
                "application/xslt+xml",
                "--to-content-type",
                "text/html",
                "--out",
                "view.html",
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("reserved"));
        assert!(stderr.contains("not yet implemented"));
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
        cli::Command::Transform(_) => run_reserved("transform", s),
        cli::Command::Schema(cli::SchemaCmd::Emit) => run_reserved("schema emit", s),
        cli::Command::Schema(cli::SchemaCmd::Sample) => run_reserved("schema sample", s),
        cli::Command::Schema(cli::SchemaCmd::Replace) => run_reserved("schema replace", s),
        cli::Command::Plugin(cli::PluginCmd::List) => run_reserved("plugin list", s),
        cli::Command::Plugin(cli::PluginCmd::Inspect) => run_reserved("plugin inspect", s),
        cli::Command::Plugin(cli::PluginCmd::Run) => run_reserved("plugin run", s),
    }
}
