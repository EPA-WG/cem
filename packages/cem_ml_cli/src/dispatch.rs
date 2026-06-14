//! CLI dispatch layer.
//!
//! Translates `cli` args into engine requests, calls the engine, serializes the
//! response, and applies exit-code policy from `cem-ml-cli-contract.md`.

#![allow(clippy::items_after_test_module)]

use crate::cli;
use crate::template_pass;
use cem_ml::engine::{self as eng, CemMlEngine, EngineError};
use cem_ml::run_config::{self, InputSpec, OutputSpec, RunConfig, RunConfigDefaults, ScopeConfig};
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
        config: Option<RunConfig>,
        diagnostics: Vec<cem_ml::diagnostics::Diagnostic>,
    },
    Engine(EngineError),
}

fn read_input(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

fn engine_input(
    path: &Path,
    from_format: Option<cli::InputFormat>,
    identity: Option<eng::FormatIdentity>,
) -> Result<eng::EngineInput, EngineError> {
    let bytes = read_input(path).map_err(|e| EngineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(eng::EngineInput {
        uri: path.display().to_string(),
        bytes,
        from_format: from_format.map(to_engine_input_format),
        identity,
        root_scope: ScopeConfig::default(),
    })
}

fn positional_input_identity(path: &Path, defaults: &ScopeConfig) -> Option<eng::FormatIdentity> {
    let mut scope = defaults.clone();
    if scope.default_content_type.is_none() {
        scope.default_content_type =
            run_config::infer_content_type_from_path(&path.display().to_string());
    }
    scope.format_identity_option()
}

fn placeholder_input(path: &Path, from_format: Option<cli::InputFormat>) -> eng::EngineInput {
    eng::EngineInput {
        uri: path.display().to_string(),
        bytes: Vec::new(),
        from_format: from_format.map(to_engine_input_format),
        identity: None,
        root_scope: ScopeConfig::default(),
    }
}

fn engine_input_from_spec(
    spec: &InputSpec,
    from_format: Option<cli::InputFormat>,
) -> Result<eng::EngineInput, CliRequestError> {
    let path = Path::new(&spec.uri);
    let bytes = read_input(path).map_err(|source| {
        CliRequestError::Engine(EngineError::Io {
            path: path.into(),
            source,
        })
    })?;
    Ok(eng::EngineInput {
        uri: spec.uri.clone(),
        bytes,
        from_format: from_format.map(to_engine_input_format),
        identity: spec.root_scope.format_identity_option(),
        root_scope: spec.root_scope.clone(),
    })
}

fn run_config(
    options: &cli::RunOptions,
    defaults: RunConfigDefaults,
) -> Result<RunConfig, CliRequestError> {
    let config_base_uri = options
        .config
        .as_ref()
        .map(|path| path.display().to_string());
    let mut config = if let Some(path) = &options.config {
        let bytes = fs::read(path).map_err(|source| {
            CliRequestError::Engine(EngineError::Io {
                path: path.clone(),
                source,
            })
        })?;
        let identity = eng::FormatIdentity {
            content_type: options
                .config_content_type
                .clone()
                .or_else(|| infer_config_content_type(path)),
            schema: None,
            base_uri: Some(path.display().to_string()),
        };
        run_config::parse_run_config(run_config::RunConfigParseRequest {
            bytes,
            identity,
            base_uri: Some(path.display().to_string()),
        })
        .map_err(|error| CliRequestError::RunConfigDiagnostics {
            config: None,
            diagnostics: vec![run_config_error_diagnostic(
                error.code,
                error.message,
                Some(path.display().to_string()),
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
            config: Some(response.config),
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
    paths: &[PathBuf],
    from_format: Option<cli::InputFormat>,
    config: &RunConfig,
    positional_defaults: &ScopeConfig,
) -> Result<Vec<eng::EngineInput>, CliRequestError> {
    let mut inputs =
        collect_inputs(paths, from_format, positional_defaults).map_err(CliRequestError::Engine)?;
    for spec in &config.inputs {
        inputs.push(engine_input_from_spec(spec, from_format)?);
    }
    Ok(inputs)
}

fn single_configured_input(
    path: Option<&Path>,
    from_format: Option<cli::InputFormat>,
    config: &RunConfig,
    positional_defaults: &ScopeConfig,
) -> Result<eng::EngineInput, CliRequestError> {
    let mut inputs = Vec::new();
    if let Some(path) = path {
        inputs.push(
            engine_input(
                path,
                from_format,
                positional_input_identity(path, positional_defaults),
            )
            .map_err(CliRequestError::Engine)?,
        );
    }
    for spec in &config.inputs {
        inputs.push(engine_input_from_spec(spec, from_format)?);
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
    paths: &[std::path::PathBuf],
    from_format: Option<cli::InputFormat>,
    positional_defaults: &ScopeConfig,
) -> Result<Vec<eng::EngineInput>, EngineError> {
    paths
        .iter()
        .map(|p| {
            engine_input(
                p,
                from_format,
                positional_input_identity(p, positional_defaults),
            )
        })
        .collect()
}

fn collect_fixture_inputs(paths: &[PathBuf]) -> Vec<eng::EngineInput> {
    if paths.is_empty() {
        default_fixture_inputs()
    } else {
        paths
            .iter()
            .map(|p| placeholder_input(p, infer_input_format(p)))
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

fn default_fixture_inputs() -> Vec<eng::EngineInput> {
    fixture_manifest_pairs()
        .into_iter()
        .flat_map(|pair| {
            [
                placeholder_input(Path::new(&pair.cem), Some(cli::InputFormat::Cem)),
                placeholder_input(Path::new(&pair.html), Some(cli::InputFormat::Html)),
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
    eng::EngineContext {
        schema: c.schema.clone(),
        content_type: c.content_type.clone(),
        base_uri: c.base_uri.clone(),
        scheduler: Default::default(),
    }
}

fn context_with_config(c: &cli::ContextOptions, config: &RunConfig) -> eng::EngineContext {
    eng::EngineContext {
        scheduler: config.scheduler.clone(),
        ..context(c)
    }
}

fn input_scope_defaults(c: &cli::ContextOptions) -> ScopeConfig {
    ScopeConfig {
        default_content_type: c.content_type.clone(),
        schema: c.schema.clone(),
        base_uri: c.base_uri.clone(),
        ..ScopeConfig::default()
    }
}

fn output_scope_defaults(args: &cli::ConvertArgs) -> ScopeConfig {
    ScopeConfig {
        default_content_type: args.to_content_type.clone(),
        schema: args.to_schema.clone(),
        base_uri: args.context.base_uri.clone(),
        ..ScopeConfig::default()
    }
}

fn run_defaults(input_scope: ScopeConfig, output_scope: ScopeConfig) -> RunConfigDefaults {
    RunConfigDefaults {
        input_scope,
        output_scope,
    }
}

fn convert_target_identity(args: &cli::ConvertArgs) -> Option<eng::FormatIdentity> {
    if args.to_content_type.is_none() && args.to_schema.is_none() && args.context.base_uri.is_none()
    {
        return None;
    }
    Some(eng::FormatIdentity {
        content_type: args.to_content_type.clone(),
        schema: args.to_schema.clone(),
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
    args: &cli::ConvertArgs,
    config: &RunConfig,
    positional_defaults: &ScopeConfig,
) -> Result<Vec<eng::EngineInput>, CliRequestError> {
    let mut inputs = Vec::new();
    if let Some(path) = args.input.as_deref() {
        inputs.push(
            engine_input(
                path,
                args.from_format,
                positional_input_identity(path, positional_defaults),
            )
            .map_err(CliRequestError::Engine)?,
        );
    }
    for spec in &config.inputs {
        inputs.push(engine_input_from_spec(spec, args.from_format)?);
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

    let inputs = match convert_configured_inputs(args, config, positional_defaults) {
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
                    context: context_with_config(&args.context, config),
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
                        if let Err(e) = write_primary(&resp.primary, args.out.as_deref(), s) {
                            let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                            return Outcome::code(EXIT_IO);
                        }
                        write_diagnostics(&resp.diagnostics, s);
                        if let Err(e) = write_convert_report_if_requested(
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
            context: context_with_config(&args.context, config),
            target: output_target_identity(output),
            target_scope: output.root_scope.clone(),
            scheduler_scope_id: index as u32,
        };
        match engine.convert(req) {
            Ok(resp) => {
                report_inputs.push(input_uri);
                report_diagnostics.extend(resp.diagnostics.clone());
                append_convert_scheduler_trace(&mut report_scheduler_trace, &resp.scheduler_trace);
                if let Err(e) = write_primary(&resp.primary, Some(destination.as_path()), s) {
                    let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                    return Outcome::code(EXIT_IO);
                }
                write_diagnostics(&resp.diagnostics, s);
            }
            Err(e) => return handle_engine_error(e, s),
        }
    }

    if let Err(e) = write_convert_report_if_requested(
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
    primary: &serde_json::Value,
    out: Option<&Path>,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    let serialized = serde_json::to_string_pretty(primary).unwrap_or_else(|_| String::new());
    match out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::write(path, serialized.as_bytes())?;
        }
        None => {
            writeln!(s.stdout, "{serialized}")?;
        }
    }
    Ok(())
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
    inputs: &[eng::EngineInput],
) -> Result<Vec<cem_ml::diagnostics::Diagnostic>, EngineError> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let from = input.from_format.unwrap_or(eng::InputFormat::Cem);
        if !matches!(from, eng::InputFormat::Cem) {
            continue;
        }
        let bytes = if input.bytes.is_empty() {
            let path = resolve_fixture_input_path(&input.uri);
            read_input(&path).map_err(|e| EngineError::Io {
                path: input.uri.clone().into(),
                source: e,
            })?
        } else {
            input.bytes.clone()
        };
        diagnostics.extend(template_pass::run(&bytes, from, Some(&input.uri)));
    }
    Ok(diagnostics)
}

fn resolve_fixture_input_path(uri: &str) -> PathBuf {
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

fn write_report_files(
    report: &cem_ml::report::Report,
    report_opts: &cli::ReportOptions,
    basename: &str,
) -> io::Result<()> {
    if let Some(p) = &report_opts.report_json {
        let target = resolve_report_target(p, basename, "json");
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&target, serde_json::to_string_pretty(report)?)?;
    }
    if let Some(p) = &report_opts.report_md {
        let target = resolve_report_target(p, basename, "md");
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&target, render_report_markdown(report))?;
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

fn run_config_diagnostic_inputs(config: Option<RunConfig>) -> Vec<String> {
    config
        .map(|config| config.inputs.into_iter().map(|input| input.uri).collect())
        .unwrap_or_default()
}

fn handle_run_config_diagnostics_report(
    config: Option<RunConfig>,
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
    if let Err(e) = write_report_files(&report, report_opts, basename) {
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
    write_report_files(&report, &args.report, REPORT_BASENAME_CONVERT)
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
    let config = match run_config(
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let input = match single_configured_input(
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
    let req = eng::ParseRequest {
        input,
        projection: to_engine_parse_projection(args.format),
        fail_level: to_engine_fail_level(args.fail_level),
        preserve_source_offsets: args.preserve_source_offsets,
        context: context_with_config(&args.context, &config),
    };
    match engine.parse(req) {
        Ok(mut resp) => {
            if let Err(e) = write_primary(&resp.primary, args.out.as_deref(), s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            resp.diagnostics.extend(embedding_diags);
            write_diagnostics(&resp.diagnostics, s);
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
    let config = match run_config(
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
    let inputs =
        match collect_configured_inputs(&args.inputs, args.from_format, &config, &input_defaults) {
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
        context: context_with_config(&args.context, &config),
    };
    match engine.validate(req) {
        Ok(mut resp) => {
            merge_embedding_diagnostics(&mut resp.report, embedding_diags);
            if let Err(e) = write_report_files(&resp.report, &args.report, REPORT_BASENAME_VALIDATE)
            {
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
    let config = match run_config(
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
    let inputs =
        match collect_configured_inputs(&args.inputs, args.from_format, &config, &input_defaults) {
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
        context: context_with_config(&args.context, &config),
    };
    match engine.check(req) {
        Ok(mut resp) => {
            merge_embedding_diagnostics(&mut resp.report, embedding_diags);
            resp.hard_violation_count = resp.report.summary.hard_violation_count;
            if let Err(e) = write_report_files(&resp.report, &args.report, REPORT_BASENAME_VALIDATE)
            {
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
    let config = match run_config(
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let input = match single_configured_input(
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
        context: context_with_config(&args.context, &config),
    };
    match engine.inspect(req) {
        Ok(resp) => {
            if let Err(e) = write_primary(&resp.body, args.out.as_deref(), s) {
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
    let config = match run_config(
        &args.run,
        run_defaults(input_defaults.clone(), output_defaults),
    ) {
        Ok(config) => config,
        Err(err) => return handle_cli_request_error(err, s),
    };
    if !config.outputs.is_empty() {
        return run_convert_fanout(engine, &args, &config, &input_defaults, s);
    }
    let input = match single_configured_input(
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
        context: context_with_config(&args.context, &config),
        target: convert_target_identity_with_config(&args, &config),
        target_scope: convert_target_scope_with_config(&args, &config),
        scheduler_scope_id: 0,
    };
    match engine.convert(req) {
        Ok(resp) => {
            let out = convert_output_destination(&args, &config);
            if let Err(e) = write_primary(&resp.primary, out.as_deref(), s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            write_diagnostics(&resp.diagnostics, s);
            if let Err(e) = write_convert_report_if_requested(
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
    let config = match run_config(
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let input = match single_configured_input(
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
        context: context_with_config(&args.context, &config),
    };
    match engine.trace(req) {
        Ok(resp) => {
            if let Err(e) = write_primary(&resp.body, args.out.as_deref(), s) {
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
    let config = match run_config(
        &args.run,
        run_defaults(input_defaults.clone(), ScopeConfig::default()),
    ) {
        Ok(config) => config,
        Err(err) => return handle_cli_request_error(err, s),
    };
    let inputs = match collect_configured_inputs(&args.inputs, None, &config, &input_defaults) {
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
        context: context_with_config(&args.context, &config),
    };
    match engine.bench(req) {
        Ok(resp) => {
            if !s.quiet {
                let json = serde_json::to_string_pretty(&resp.body).unwrap_or_default();
                let _ = writeln!(s.stdout, "{json}");
            }
            if let Some(p) = &args.report.report_json {
                if let Err(e) = (|| -> io::Result<()> {
                    let target = resolve_report_target(p, REPORT_BASENAME_BENCH, "json");
                    if let Some(parent) = target.parent() {
                        if !parent.as_os_str().is_empty() {
                            fs::create_dir_all(parent)?;
                        }
                    }
                    fs::write(&target, serde_json::to_string_pretty(&resp.body)?)
                })() {
                    let _ = writeln!(s.stderr, "cem-ml: bench report write failure: {e}");
                    return Outcome::code(EXIT_IO);
                }
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
    let config = match run_config(
        &args.run,
        run_defaults(input_defaults, ScopeConfig::default()),
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
    let mut inputs = collect_fixture_inputs(&args.inputs);
    for spec in &config.inputs {
        match engine_input_from_spec(spec, None) {
            Ok(input) => inputs.push(input),
            Err(err) => return handle_cli_request_error(err, s),
        }
    }
    let embedding_diags = match collect_fixture_embedding_diagnostics(&inputs) {
        Ok(v) => v,
        Err(e) => return handle_engine_error(e, s),
    };
    let req = eng::FixtureValidateRequest {
        inputs,
        fail_level: to_engine_fail_level(args.fail_level),
        zero_hard_violations: args.zero_hard_violations,
        context: context_with_config(&args.context, &config),
    };
    match engine.fixture_validate(req) {
        Ok(mut resp) => {
            merge_embedding_diagnostics(&mut resp.report, embedding_diags);
            if let Err(e) = write_report_files(&resp.report, &args.report, REPORT_BASENAME_VALIDATE)
            {
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
    let config = match run_config(
        &args.run,
        run_defaults(input_defaults, ScopeConfig::default()),
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
    let mut inputs = collect_fixture_inputs(&args.inputs);
    for spec in &config.inputs {
        match engine_input_from_spec(spec, None) {
            Ok(input) => inputs.push(input),
            Err(err) => return handle_cli_request_error(err, s),
        }
    }
    let req = eng::FixtureRoundtripRequest {
        inputs,
        to_format: to_engine_layer_format(args.to_format),
        context: context_with_config(&args.context, &config),
    };
    match engine.fixture_roundtrip(req) {
        Ok(resp) => {
            if let Err(e) =
                write_report_files(&resp.report, &args.report, REPORT_BASENAME_ROUNDTRIP)
            {
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
        "cem-ml: `{name}` is reserved until its subsystem plan exists (exit 2 per cem-ml-cli-contract.md)."
    );
    Outcome::code(EXIT_USAGE_OR_RESERVED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::engine::NotImplementedEngine;
    use cem_ml::fake::FakeEngine;
    use cem_ml::real::RealCemMlEngine;
    use clap::Parser;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn parse_cli(args: &[&str]) -> cli::Cli {
        cli::Cli::try_parse_from(std::iter::once("cem-ml").chain(args.iter().copied())).unwrap()
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

    #[test]
    fn version_subcommand_prints_version_and_exits_zero() {
        let (outcome, stdout, _) = run(&NotImplementedEngine, &["version"]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        assert!(stdout.starts_with("cem-ml "));
        assert!(stdout.contains(cli::COPYRIGHT_NOTICE));
    }

    #[test]
    fn reserved_transform_exits_two() {
        let (outcome, _, stderr) = run(&NotImplementedEngine, &["transform"]);
        assert_eq!(outcome.exit_code, EXIT_USAGE_OR_RESERVED);
        assert!(stderr.contains("reserved"));
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
    fn parse_with_fake_engine_emits_json_to_stdout() {
        let p = write_fixture("parse-fake.cem", "{x}");
        let (outcome, stdout, _) = run(&FakeEngine, &["parse", p.to_str().unwrap()]);
        assert_eq!(outcome.exit_code, EXIT_OK);
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
        assert_eq!(v["kind"], "fake-parse");
        assert_eq!(v["projection"], "dom-json");
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
                    "uri={},namespaces=html:https://www.w3.org/1999/xhtml,budgets=parseMs:5",
                    p.display()
                ),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK, "{stderr}");
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let diagnostics = v["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.namespaces_unenforced"));
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn positional_input_identity_infers_content_type_from_extension() {
        let identity =
            positional_input_identity(Path::new("src/screen.html"), &ScopeConfig::default())
                .expect("html extension should infer content type");
        assert_eq!(identity.content_type.as_deref(), Some("text/html"));
        assert_eq!(identity.schema, None);
        assert_eq!(identity.base_uri, None);
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
        let inputs = collect_fixture_inputs(&[
            PathBuf::from("examples/cem-ml/login.cem"),
            PathBuf::from("examples/semantic/login.html"),
            PathBuf::from("examples/cem-ml/namespace-rebinding/default-html-svg-html.xml"),
        ]);
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
}

fn observable_inputs(
    command: &cli::Command,
) -> Result<(Vec<eng::EngineInput>, eng::EngineContext), CliRequestError> {
    match command {
        cli::Command::Parse(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let input = single_configured_input(
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context_with_config(&a.context, &config)))
        }
        cli::Command::Validate(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let inputs =
                collect_configured_inputs(&a.inputs, a.from_format, &config, &input_defaults)?;
            Ok((inputs, context_with_config(&a.context, &config)))
        }
        cli::Command::Check(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let inputs =
                collect_configured_inputs(&a.inputs, a.from_format, &config, &input_defaults)?;
            Ok((inputs, context_with_config(&a.context, &config)))
        }
        cli::Command::Inspect(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let input = single_configured_input(
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context_with_config(&a.context, &config)))
        }
        cli::Command::Convert(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let output_defaults = output_scope_defaults(a);
            let config = run_config(
                &a.run,
                run_defaults(input_defaults.clone(), output_defaults),
            )?;
            let input = single_configured_input(
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context_with_config(&a.context, &config)))
        }
        cli::Command::Trace(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let input = single_configured_input(
                a.input.as_deref(),
                a.from_format,
                &config,
                &input_defaults,
            )?;
            Ok((vec![input], context_with_config(&a.context, &config)))
        }
        cli::Command::Bench(a) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(
                &a.run,
                run_defaults(input_defaults.clone(), ScopeConfig::default()),
            )?;
            let inputs = collect_configured_inputs(&a.inputs, None, &config, &input_defaults)?;
            Ok((inputs, context_with_config(&a.context, &config)))
        }
        cli::Command::Fixture(cli::FixtureCmd::Validate(a)) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(&a.run, run_defaults(input_defaults, ScopeConfig::default()))?;
            let mut inputs = collect_fixture_inputs(&a.inputs);
            for spec in &config.inputs {
                inputs.push(engine_input_from_spec(spec, None)?);
            }
            Ok((inputs, context_with_config(&a.context, &config)))
        }
        cli::Command::Fixture(cli::FixtureCmd::Roundtrip(a)) => {
            let input_defaults = input_scope_defaults(&a.context);
            let config = run_config(&a.run, run_defaults(input_defaults, ScopeConfig::default()))?;
            let mut inputs = collect_fixture_inputs(&a.inputs);
            for spec in &config.inputs {
                inputs.push(engine_input_from_spec(spec, None)?);
            }
            Ok((inputs, context_with_config(&a.context, &config)))
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
            let path = resolve_fixture_input_path(&input.uri);
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    let _ = writeln!(
                        s.stderr,
                        "cem-ml: --observe-events: cannot read {}: {e}",
                        input.uri
                    );
                    continue;
                }
            };
            eng::EngineInput { bytes, ..input }
        } else {
            input
        };
        let loaded = registry.load(&input, &context);
        let observer = cem_ml::observability::BufferingObserver::new();
        let _ = cem_ml::real::observe_pipeline(&loaded.bytes, loaded.from_format, &observer);
        all_events.extend(observer.drain());
    }

    let jsonl = cem_ml::observability::events_to_jsonl(&all_events);
    if target.as_os_str() == "-" {
        s.stdout.write_all(jsonl.as_bytes())?;
        s.stdout.flush()?;
    } else {
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(target, jsonl.as_bytes())?;
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
        cli::Command::Transform => run_reserved("transform", s),
        cli::Command::Schema(cli::SchemaCmd::Emit) => run_reserved("schema emit", s),
        cli::Command::Schema(cli::SchemaCmd::Sample) => run_reserved("schema sample", s),
        cli::Command::Schema(cli::SchemaCmd::Replace) => run_reserved("schema replace", s),
        cli::Command::Plugin(cli::PluginCmd::List) => run_reserved("plugin list", s),
        cli::Command::Plugin(cli::PluginCmd::Inspect) => run_reserved("plugin inspect", s),
        cli::Command::Plugin(cli::PluginCmd::Run) => run_reserved("plugin run", s),
    }
}
