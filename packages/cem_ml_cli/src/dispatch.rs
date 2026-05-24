//! CLI dispatch layer.
//!
//! Translates `cli` args into engine requests, calls the engine, serializes the
//! response, and applies exit-code policy from `cem-ml-cli-contract.md`.

#![allow(clippy::items_after_test_module)]

use crate::cli;
use crate::template_pass;
use cem_ml::engine::{self as eng, CemMlEngine, EngineError};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

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

fn read_input(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

fn engine_input(
    path: &Path,
    from_format: Option<cli::InputFormat>,
) -> Result<eng::EngineInput, EngineError> {
    let bytes = read_input(path).map_err(|e| EngineError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(eng::EngineInput {
        uri: path.display().to_string(),
        bytes,
        from_format: from_format.map(to_engine_input_format),
    })
}

fn placeholder_input(path: &Path, from_format: Option<cli::InputFormat>) -> eng::EngineInput {
    eng::EngineInput {
        uri: path.display().to_string(),
        bytes: Vec::new(),
        from_format: from_format.map(to_engine_input_format),
    }
}

fn collect_inputs(
    paths: &[std::path::PathBuf],
    from_format: Option<cli::InputFormat>,
) -> Result<Vec<eng::EngineInput>, EngineError> {
    paths.iter().map(|p| engine_input(p, from_format)).collect()
}

fn collect_fixture_inputs(paths: &[std::path::PathBuf]) -> Vec<eng::EngineInput> {
    let resolved: Vec<&Path> = if paths.is_empty() {
        DEFAULT_FIXTURES.iter().map(Path::new).collect()
    } else {
        paths.iter().map(|p| p.as_path()).collect()
    };
    resolved
        .into_iter()
        .map(|p| placeholder_input(p, None))
        .collect()
}

const DEFAULT_FIXTURES: &[&str] = &[
    "examples/cem-ml/login.cem",
    "examples/cem-ml/registration.cem",
    "examples/cem-ml/profile.cem",
    "examples/cem-ml/assets-list.cem",
    "examples/cem-ml/message-thread.cem",
    "examples/semantic/login.html",
    "examples/semantic/registration.html",
    "examples/semantic/profile.html",
    "examples/semantic/assets-list.html",
    "examples/semantic/message-thread.html",
];

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
fn collect_embedding_diagnostics(inputs: &[eng::EngineInput]) -> Vec<cem_ml::diagnostics::Diagnostic> {
    let mut diagnostics = Vec::new();
    for input in inputs {
        let from = input.from_format.unwrap_or(eng::InputFormat::Cem);
        diagnostics.extend(template_pass::run(&input.bytes, from, Some(&input.uri)));
    }
    diagnostics
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
    let input = match engine_input(&args.input, args.from_format) {
        Ok(i) => i,
        Err(e) => return handle_engine_error(e, s),
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
        context: context(&args.context),
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
    let inputs = match collect_inputs(&args.inputs, args.from_format) {
        Ok(v) => v,
        Err(e) => return handle_engine_error(e, s),
    };
    let embedding_diags = collect_embedding_diagnostics(&inputs);
    let req = eng::ValidateRequest {
        inputs,
        projection: to_engine_validate_projection(args.format),
        fail_level: to_engine_fail_level(args.fail_level),
        context: context(&args.context),
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
    let inputs = match collect_inputs(&args.inputs, args.from_format) {
        Ok(v) => v,
        Err(e) => return handle_engine_error(e, s),
    };
    let embedding_diags = collect_embedding_diagnostics(&inputs);
    let req = eng::CheckRequest {
        inputs,
        projection: to_engine_validate_projection(args.format),
        fail_level: to_engine_fail_level(args.fail_level),
        zero_hard_violations: args.zero_hard_violations,
        context: context(&args.context),
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
    let input = match engine_input(&args.input, args.from_format) {
        Ok(i) => i,
        Err(e) => return handle_engine_error(e, s),
    };
    let req = eng::InspectRequest {
        input,
        show: to_engine_inspect_view(args.show),
        context: context(&args.context),
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
    let input = match engine_input(&args.input, args.from_format) {
        Ok(i) => i,
        Err(e) => return handle_engine_error(e, s),
    };
    let req = eng::ConvertRequest {
        input,
        to_format: to_engine_layer_format(args.to_format),
        preserve_source_offsets: args.preserve_source_offsets,
        context: context(&args.context),
    };
    match engine.convert(req) {
        Ok(resp) => {
            if let Err(e) = write_primary(&resp.primary, args.out.as_deref(), s) {
                let _ = writeln!(s.stderr, "cem-ml: write failure: {e}");
                return Outcome::code(EXIT_IO);
            }
            write_diagnostics(&resp.diagnostics, s);
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
    let input = match engine_input(&args.input, args.from_format) {
        Ok(i) => i,
        Err(e) => return handle_engine_error(e, s),
    };
    let req = eng::TraceRequest {
        input,
        projection: to_engine_trace_projection(args.format),
        context: context(&args.context),
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
    let inputs = match collect_inputs(&args.inputs, None) {
        Ok(v) => v,
        Err(e) => return handle_engine_error(e, s),
    };
    let req = eng::BenchRequest {
        inputs,
        projection: to_engine_bench_projection(args.format),
        iterations: args.iterations,
        budget_ms: args.budget_ms,
        profile: args.profile.map(to_engine_bench_profile),
        cold_cache: args.cold_cache,
        context: context(&args.context),
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
    let inputs = collect_fixture_inputs(&args.inputs);
    let req = eng::FixtureValidateRequest {
        inputs,
        fail_level: to_engine_fail_level(args.fail_level),
        zero_hard_violations: args.zero_hard_violations,
        context: context(&args.context),
    };
    match engine.fixture_validate(req) {
        Ok(resp) => {
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
    let inputs = collect_fixture_inputs(&args.inputs);
    let req = eng::FixtureRoundtripRequest {
        inputs,
        to_format: to_engine_layer_format(args.to_format),
        context: context(&args.context),
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
        assert!(count >= 10, "expected default fixture set, got {count}");
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
            &[
                "--observe-events",
                "-",
                "parse",
                p.to_str().unwrap(),
            ],
        );
        assert_eq!(outcome.exit_code, EXIT_OK);
        // Stdout carries the JSONL events stream plus the normal
        // parse projection JSON. The first non-empty line should parse
        // as a JSONL event.
        let first = stdout.lines().next().expect("at least one output line");
        let v: serde_json::Value =
            serde_json::from_str(first).expect("first line of stdout is JSONL");
        assert!(v.get("channel").is_some(), "channel field must be present");
        assert!(v.get("sequence").is_some(), "sequence field must be present");
    }
}

/// Pull the input paths that the chosen subcommand would feed through
/// the pipeline. Subcommands that do not consume a CEM-ML document
/// (`version`, `fixture roundtrip`'s metadata-only shape, `transform`)
/// yield an empty slice, which suppresses event emission.
fn observable_inputs(command: &cli::Command) -> Vec<(std::path::PathBuf, Option<cli::InputFormat>)> {
    match command {
        cli::Command::Parse(a) => vec![(a.input.clone(), a.from_format)],
        cli::Command::Validate(a) => a
            .inputs
            .iter()
            .map(|p| (p.clone(), a.from_format))
            .collect(),
        cli::Command::Check(a) => a
            .inputs
            .iter()
            .map(|p| (p.clone(), a.from_format))
            .collect(),
        cli::Command::Inspect(a) => vec![(a.input.clone(), a.from_format)],
        cli::Command::Convert(a) => vec![(a.input.clone(), None)],
        cli::Command::Trace(a) => vec![(a.input.clone(), None)],
        _ => Vec::new(),
    }
}

fn emit_observability_events(
    command: &cli::Command,
    target: &Path,
    s: &mut Streams<'_>,
) -> io::Result<()> {
    let inputs = observable_inputs(command);
    if inputs.is_empty() {
        return Ok(());
    }

    let mut all_events: Vec<cem_ml::observability::ReportEvent> = Vec::new();
    for (path, from_format) in inputs {
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                let _ = writeln!(
                    s.stderr,
                    "cem-ml: --observe-events: cannot read {}: {e}",
                    path.display()
                );
                continue;
            }
        };
        let from = from_format
            .map(to_engine_input_format)
            .unwrap_or(cem_ml::engine::InputFormat::Cem);
        let observer = cem_ml::observability::BufferingObserver::new();
        let _ = cem_ml::real::observe_pipeline(&bytes, from, &observer);
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
