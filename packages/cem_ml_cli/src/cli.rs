use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::str::FromStr;

pub const COPYRIGHT_NOTICE: &str =
    "Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>";

#[derive(Parser, Debug)]
#[command(
    name = "cem-ml",
    bin_name = "cem-ml",
    about = "CEM parser/runtime CLI",
    long_about = "CEM parser/runtime CLI. See docs/cem-ml-cli-contract.md for the feature surface.

Repository: https://github.com/EPA-WG/cem
Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>",
    version = cem_ml::VERSION,
    propagate_version = true,
    disable_help_subcommand = false,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(
        long,
        global = true,
        conflicts_with = "verbose",
        help = "Suppress success/info output (errors still surface)"
    )]
    pub quiet: bool,

    #[arg(long, global = true, help = "Emit verbose progress and trace text")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Disable ANSI color in terminal output")]
    pub no_color: bool,

    #[arg(
        long = "observe-events",
        global = true,
        value_name = "PATH",
        help = "Write the structured observability event stream (parse/validate/transform) to PATH as JSONL; use - for stdout"
    )]
    pub observe_events: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Parse(ParseArgs),
    Validate(ValidateArgs),
    Check(CheckArgs),
    Inspect(InspectArgs),
    Convert(ConvertArgs),
    Trace(TraceArgs),
    Bench(BenchArgs),
    #[command(subcommand)]
    Fixture(FixtureCmd),
    #[command(about = "Print the cem-ml-cli version")]
    Version,

    #[command(about = "Apply a template/stylesheet to data")]
    Transform(TransformArgs),
    #[command(subcommand, about = "Reserved: schema workflows (not yet implemented)")]
    Schema(SchemaCmd),
    #[command(subcommand, about = "Reserved: plugin workflows (not yet implemented)")]
    Plugin(PluginCmd),
}

#[derive(Subcommand, Debug)]
pub enum FixtureCmd {
    #[command(about = "Validate canonical CEM-ML fixtures and HTML parity fixtures")]
    Validate(FixtureValidateArgs),
    #[command(about = "Round-trip fixtures through parser-backed projections")]
    Roundtrip(FixtureRoundtripArgs),
}

#[derive(Subcommand, Debug)]
pub enum SchemaCmd {
    Emit,
    Sample,
    Replace,
}

#[derive(Subcommand, Debug)]
pub enum PluginCmd {
    List,
    Inspect,
    Run,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum FailLevel {
    Parse,
    Validate,
    Strict,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputFormat {
    Cem,
    Html,
    Xml,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayerFormat {
    Cem,
    Html,
    Xml,
    DomJson,
    Ast,
    Events,
    DomBin,
    AstBin,
    EventsBin,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParseFormat {
    DomJson,
    Json,
    Ast,
    Events,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum ValidateFormat {
    Json,
    Xml,
    Cem,
    Text,
    Html,
    Markdown,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum TraceFormat {
    Json,
    Xml,
    Cem,
    Text,
    Html,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum BenchFormat {
    Text,
    Json,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum InspectView {
    Summary,
    Ast,
    Events,
    Diagnostics,
    SourceOffsets,
    Tree,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum BenchProfile {
    Cpu,
    Memory,
}

#[derive(Args, Debug, Default, Clone)]
pub struct ContextOptions {
    #[arg(
        long,
        value_name = "URI-OR-FILE",
        help = "Schema URI or file to record on diagnostics/reports"
    )]
    pub schema: Option<String>,

    #[arg(
        long,
        value_name = "TYPE",
        help = "Content type to record on diagnostics/reports"
    )]
    pub content_type: Option<String>,

    #[arg(
        long = "default-namespace",
        value_name = "URI",
        help = "Default namespace URI for the input root scope"
    )]
    pub default_namespace: Option<String>,

    #[arg(
        long = "namespace",
        value_name = "PREFIX=URI",
        action = clap::ArgAction::Append,
        help = "Named namespace binding for the input root scope; repeatable"
    )]
    pub namespaces: Vec<NamespaceBinding>,

    #[arg(
        long = "module-map",
        value_name = "PATH-OR-URI",
        help = "Module-map path or URI for the input root scope"
    )]
    pub module_map: Option<String>,

    #[arg(
        long = "version-pin",
        value_name = "NAME=CONSTRAINT",
        action = clap::ArgAction::Append,
        help = "Version pin for the input root scope; repeatable"
    )]
    pub version_pins: Vec<ScopeKeyValue>,

    #[arg(
        long = "scope-policy",
        value_name = "NAME",
        help = "Scheduler/resource policy name for the input root scope"
    )]
    pub scope_policy: Option<String>,

    #[arg(
        long = "scope-budget",
        value_name = "NAME=VALUE",
        action = clap::ArgAction::Append,
        help = "Resource budget for the input root scope; repeatable"
    )]
    pub scope_budgets: Vec<ScopeKeyValue>,

    #[arg(
        long,
        value_name = "URI",
        help = "Base URI for diagnostic/report URI normalization"
    )]
    pub base_uri: Option<String>,

    #[arg(
        long = "resolver-read-map",
        value_name = "URI-PREFIX=DIR",
        action = clap::ArgAction::Append,
        help = "Resolve matching remote/custom read URIs from a local directory; repeatable"
    )]
    pub resolver_read_maps: Vec<ResolverMap>,

    #[arg(
        long = "resolver-write-map",
        value_name = "URI-PREFIX=DIR",
        action = clap::ArgAction::Append,
        help = "Resolve matching remote/custom write URIs to a local directory; repeatable"
    )]
    pub resolver_write_maps: Vec<ResolverMap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceBinding {
    pub prefix: String,
    pub uri: String,
}

impl FromStr for NamespaceBinding {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((prefix, uri)) = value.split_once('=') else {
            return Err("expected PREFIX=URI".to_owned());
        };
        let prefix = prefix.trim();
        let uri = uri.trim();
        if prefix.is_empty() {
            return Err("namespace prefix must not be empty; use --default-namespace for the default namespace".to_owned());
        }
        if uri.is_empty() {
            return Err("namespace URI must not be empty".to_owned());
        }
        Ok(Self {
            prefix: prefix.to_owned(),
            uri: uri.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolverMap {
    pub uri_prefix: String,
    pub local_root: PathBuf,
}

impl FromStr for ResolverMap {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((uri_prefix, local_root)) = value.split_once('=') else {
            return Err("expected URI-PREFIX=DIR".to_owned());
        };
        let uri_prefix = uri_prefix.trim();
        let local_root = local_root.trim();
        if uri_prefix.is_empty() {
            return Err("resolver URI prefix must not be empty".to_owned());
        }
        if local_root.is_empty() {
            return Err("resolver local root must not be empty".to_owned());
        }
        if cem_ml::resolver::uri_scheme(uri_prefix).is_none()
            || cem_ml::resolver::is_windows_drive_path(uri_prefix)
        {
            return Err("resolver URI prefix must include a remote/custom URI scheme".to_owned());
        }
        Ok(Self {
            uri_prefix: uri_prefix.to_owned(),
            local_root: PathBuf::from(local_root),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeKeyValue {
    pub key: String,
    pub value: String,
}

impl FromStr for ScopeKeyValue {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((key, field_value)) = value.split_once('=') else {
            return Err("expected NAME=VALUE".to_owned());
        };
        let key = key.trim();
        let field_value = field_value.trim();
        if key.is_empty() {
            return Err("name must not be empty".to_owned());
        }
        if field_value.is_empty() {
            return Err("value must not be empty".to_owned());
        }
        Ok(Self {
            key: key.to_owned(),
            value: field_value.to_owned(),
        })
    }
}

#[derive(Args, Debug, Default, Clone)]
pub struct ReportOptions {
    #[arg(
        long,
        value_name = "FILE-OR-DIR",
        help = "Write JSON report to file or default name in dir"
    )]
    pub report_json: Option<PathBuf>,

    #[arg(
        long,
        value_name = "FILE-OR-DIR",
        help = "Write Markdown report to file or default name in dir"
    )]
    pub report_md: Option<PathBuf>,
}

#[derive(Args, Debug, Default, Clone)]
pub struct RunOptions {
    #[arg(
        long = "config",
        value_name = "FILE",
        help = "Read structured run configuration JSON with inputs, outputs, and scheduler settings"
    )]
    pub config: Option<PathBuf>,

    #[arg(
        long = "config-content-type",
        value_name = "TYPE",
        help = "Content type of --config; inferred from extension when omitted"
    )]
    pub config_content_type: Option<String>,

    #[arg(
        long = "config-schema",
        value_name = "URI",
        help = "Schema identity of --config"
    )]
    pub config_schema: Option<String>,

    #[arg(
        long = "input-spec",
        value_name = "CSV",
        action = clap::ArgAction::Append,
        help = "Repeatable input spec record, e.g. uri=src/a.cem,contentType=application/cem+xml,schema=core"
    )]
    pub input_specs: Vec<String>,

    #[arg(
        long = "output-spec",
        value_name = "CSV",
        action = clap::ArgAction::Append,
        help = "Repeatable output spec record, e.g. input=src/a.cem,dest=dist/a.cem,contentType=application/cem+xml"
    )]
    pub output_specs: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ParseArgs {
    #[arg(value_name = "INPUT", help = "Path to a CEM-ML/HTML/XML input")]
    pub input: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = ParseFormat::DomJson,
          help = "Output projection (dom-json|json|ast|events)")]
    pub format: ParseFormat,

    #[arg(
        long = "from-format",
        value_enum,
        help = "Override input format detection"
    )]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_enum, default_value_t = FailLevel::Parse)]
    pub fail_level: FailLevel,

    #[arg(
        long,
        value_name = "FILE",
        help = "Write primary output to file (stdout if omitted)"
    )]
    pub out: Option<PathBuf>,

    #[arg(long, help = "Preserve absolute source byte offsets in output")]
    pub preserve_source_offsets: bool,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    #[arg(value_name = "INPUT", num_args = 0.., help = "One or more inputs")]
    pub inputs: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = ValidateFormat::Text,
          help = "Report projection (json|xml|cem|text|html|markdown)")]
    pub format: ValidateFormat,

    #[arg(long = "from-format", value_enum)]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_enum, default_value_t = FailLevel::Validate)]
    pub fail_level: FailLevel,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    #[arg(value_name = "INPUT", num_args = 0..)]
    pub inputs: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = ValidateFormat::Text)]
    pub format: ValidateFormat,

    #[arg(long = "from-format", value_enum)]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_enum, default_value_t = FailLevel::Validate)]
    pub fail_level: FailLevel,

    #[arg(long, help = "Exit non-zero if any hard violations exist")]
    pub zero_hard_violations: bool,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = InspectView::Summary,
          help = "Which inspector view to render")]
    pub show: InspectView,

    #[arg(long = "from-format", value_enum)]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
}

#[derive(Args, Debug)]
pub struct ConvertArgs {
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    #[arg(long = "from-format", value_enum, help = "Input syntax (cem|html|xml)")]
    pub from_format: Option<InputFormat>,

    #[arg(long = "to-format", value_enum, default_value_t = LayerFormat::DomJson,
          help = "Output layer (cem|html|xml|dom-json|ast|events|dom-bin|ast-bin|events-bin)")]
    pub to_format: LayerFormat,

    #[arg(
        long = "to-content-type",
        value_name = "TYPE",
        help = "Target content type for conversion/export"
    )]
    pub to_content_type: Option<String>,

    #[arg(
        long = "to-schema",
        value_name = "URI-OR-FILE",
        help = "Target schema URI or file for conversion/export"
    )]
    pub to_schema: Option<String>,

    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    #[arg(long, help = "Preserve absolute source byte offsets in output")]
    pub preserve_source_offsets: bool,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct TransformArgs {
    #[arg(
        value_name = "DATA",
        required_unless_present = "config",
        help = "Path to source data for the template"
    )]
    pub data: Option<PathBuf>,

    #[arg(
        long = "config",
        value_name = "FILE",
        help = "Read CEM-ML transform graph configuration"
    )]
    pub config: Option<PathBuf>,

    #[arg(
        long = "config-content-type",
        value_name = "TYPE",
        help = "Content type of --config; inferred from extension when omitted"
    )]
    pub config_content_type: Option<String>,

    #[arg(
        long = "config-schema",
        value_name = "URI",
        help = "Schema identity of --config"
    )]
    pub config_schema: Option<String>,

    #[arg(
        long = "data-content-type",
        value_name = "TYPE",
        help = "Content type of DATA"
    )]
    pub data_content_type: Option<String>,

    #[arg(
        long = "data-schema",
        value_name = "URI-OR-FILE",
        help = "Schema URI or file for DATA"
    )]
    pub data_schema: Option<String>,

    #[arg(
        long = "template",
        value_name = "FILE",
        required_unless_present = "config",
        help = "Template or stylesheet to apply to DATA"
    )]
    pub template: Option<PathBuf>,

    #[arg(
        long = "template-content-type",
        value_name = "TYPE",
        help = "Content type of --template"
    )]
    pub template_content_type: Option<String>,

    #[arg(
        long = "template-schema",
        value_name = "URI-OR-FILE",
        help = "Schema URI or file for --template"
    )]
    pub template_schema: Option<String>,

    #[arg(
        long = "template-entrypoint",
        value_name = "NAME",
        help = "Public CEM-native template entrypoint to render"
    )]
    pub template_entrypoint: Option<String>,

    #[arg(
        long = "param",
        value_name = "NAME=VALUE",
        help = "CEM-native template param; repeatable"
    )]
    pub params: Vec<String>,

    #[arg(
        long = "to-content-type",
        value_name = "TYPE",
        help = "Target document content type"
    )]
    pub to_content_type: Option<String>,

    #[arg(
        long = "to-schema",
        value_name = "URI-OR-FILE",
        help = "Target schema URI or file"
    )]
    pub to_schema: Option<String>,

    #[arg(long, value_name = "FILE", help = "Write target document to file")]
    pub out: Option<PathBuf>,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct TraceArgs {
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = TraceFormat::Json,
          help = "Trace projection (json|xml|cem|text|html)")]
    pub format: TraceFormat,

    #[arg(long = "from-format", value_enum)]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
}

#[derive(Args, Debug)]
pub struct BenchArgs {
    #[arg(value_name = "INPUT", num_args = 0..)]
    pub inputs: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = BenchFormat::Text,
          help = "Bench report projection (text|json)")]
    pub format: BenchFormat,

    #[arg(long, value_name = "N", default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..),
          help = "Number of iterations (>=1)")]
    pub iterations: u32,

    #[arg(long = "budget-ms", value_name = "MS",
          value_parser = clap::value_parser!(u64).range(1..),
          help = "Fail when per-iteration wall time exceeds this budget")]
    pub budget_ms: Option<u64>,

    #[arg(long, value_enum, help = "Optional profiling mode")]
    pub profile: Option<BenchProfile>,

    #[arg(long = "cold-cache", help = "Reset caches between iterations")]
    pub cold_cache: bool,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct FixtureValidateArgs {
    #[arg(value_name = "INPUT", num_args = 0..,
          help = "Fixtures to validate; defaults to canonical CEM-ML + HTML parity fixtures")]
    pub inputs: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = FailLevel::Validate)]
    pub fail_level: FailLevel,

    #[arg(long, help = "Exit non-zero if any hard violations exist")]
    pub zero_hard_violations: bool,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct FixtureRoundtripArgs {
    #[arg(value_name = "INPUT", num_args = 0..,
          help = "Fixtures to round-trip; defaults to canonical CEM-ML + HTML parity fixtures")]
    pub inputs: Vec<PathBuf>,

    #[arg(long = "to-format", value_enum, default_value_t = LayerFormat::DomJson)]
    pub to_format: LayerFormat,

    #[command(flatten)]
    pub context: ContextOptions,
    #[command(flatten)]
    pub run: RunOptions,
    #[command(flatten)]
    pub report: ReportOptions,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_definition_is_well_formed() {
        Cli::command().debug_assert();
    }

    fn try_parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("cem-ml").chain(args.iter().copied()))
    }

    #[test]
    fn parse_accepts_layer_formats_only() {
        for fmt in ["dom-json", "json", "ast", "events"] {
            try_parse(&["parse", "--format", fmt, "in.cem"]).expect(fmt);
        }
        for fmt in ["xml", "cem", "text", "html", "markdown", "tree"] {
            assert!(
                try_parse(&["parse", "--format", fmt, "in.cem"]).is_err(),
                "rejected: {fmt}"
            );
        }
    }

    #[test]
    fn validate_accepts_report_formats_only() {
        for fmt in ["json", "xml", "cem", "text", "html", "markdown"] {
            try_parse(&["validate", "--format", fmt, "in.cem"]).expect(fmt);
        }
        for fmt in ["dom-json", "ast", "events", "tree"] {
            assert!(
                try_parse(&["validate", "--format", fmt, "in.cem"]).is_err(),
                "rejected: {fmt}"
            );
        }
    }

    #[test]
    fn check_accepts_report_formats_only() {
        try_parse(&["check", "--format", "json", "in.cem"]).unwrap();
        assert!(try_parse(&["check", "--format", "ast", "in.cem"]).is_err());
    }

    #[test]
    fn trace_accepts_trace_formats_only() {
        for fmt in ["json", "xml", "cem", "text", "html"] {
            try_parse(&["trace", "--format", fmt, "in.cem"]).expect(fmt);
        }
        for fmt in ["markdown", "dom-json", "ast", "events", "tree"] {
            assert!(
                try_parse(&["trace", "--format", fmt, "in.cem"]).is_err(),
                "rejected: {fmt}"
            );
        }
    }

    #[test]
    fn bench_accepts_text_or_json_only() {
        try_parse(&["bench", "--format", "text", "in.cem"]).unwrap();
        try_parse(&["bench", "--format", "json", "in.cem"]).unwrap();
        for fmt in [
            "xml", "cem", "html", "markdown", "dom-json", "ast", "events", "tree",
        ] {
            assert!(
                try_parse(&["bench", "--format", fmt, "in.cem"]).is_err(),
                "rejected: {fmt}"
            );
        }
    }

    #[test]
    fn inspect_accepts_documented_views() {
        for view in [
            "summary",
            "ast",
            "events",
            "diagnostics",
            "source-offsets",
            "tree",
        ] {
            try_parse(&["inspect", "--show", view, "in.cem"]).expect(view);
        }
        assert!(try_parse(&["inspect", "--show", "scope", "in.cem"]).is_err());
    }

    #[test]
    fn convert_to_format_restricted_to_layer_formats() {
        for fmt in [
            "cem",
            "html",
            "xml",
            "dom-json",
            "ast",
            "events",
            "dom-bin",
            "ast-bin",
            "events-bin",
        ] {
            try_parse(&["convert", "--to-format", fmt, "in.cem"]).expect(fmt);
        }
        for fmt in ["json", "text"] {
            assert!(
                try_parse(&["convert", "--to-format", fmt, "in.cem"]).is_err(),
                "rejected: {fmt}"
            );
        }
    }

    #[test]
    fn convert_accepts_target_identity_flags() {
        try_parse(&[
            "convert",
            "--to-format",
            "cem",
            "--to-content-type",
            "application/cem+xml",
            "--to-schema",
            "https://cem.dev/ns/core/1",
            "in.html",
        ])
        .unwrap();
    }

    #[test]
    fn convert_document_to_document_example_parses() {
        let cli = try_parse(&[
            "convert",
            "input.xml",
            "--content-type",
            "application/xml",
            "--to-content-type",
            "application/cem+xml",
            "--out",
            "output.cem",
        ])
        .unwrap();

        let Command::Convert(args) = cli.command else {
            panic!("expected convert command");
        };
        assert_eq!(args.input, Some(PathBuf::from("input.xml")));
        assert_eq!(
            args.context.content_type.as_deref(),
            Some("application/xml")
        );
        assert_eq!(args.to_content_type.as_deref(), Some("application/cem+xml"));
        assert_eq!(args.out, Some(PathBuf::from("output.cem")));
    }

    #[test]
    fn transform_template_shape_parses() {
        let cli = try_parse(&[
            "transform",
            "data.xml",
            "--data-content-type",
            "application/xml",
            "--data-schema",
            "data.rng",
            "--template",
            "view.xsl",
            "--template-content-type",
            "application/xslt+xml",
            "--template-schema",
            "xslt.rng",
            "--to-content-type",
            "text/html",
            "--to-schema",
            "html.rng",
            "--out",
            "view.html",
        ])
        .unwrap();

        let Command::Transform(args) = cli.command else {
            panic!("expected transform command");
        };
        assert_eq!(args.data, Some(PathBuf::from("data.xml")));
        assert_eq!(args.data_content_type.as_deref(), Some("application/xml"));
        assert_eq!(args.data_schema.as_deref(), Some("data.rng"));
        assert_eq!(args.template, Some(PathBuf::from("view.xsl")));
        assert_eq!(
            args.template_content_type.as_deref(),
            Some("application/xslt+xml")
        );
        assert_eq!(args.template_schema.as_deref(), Some("xslt.rng"));
        assert_eq!(args.to_content_type.as_deref(), Some("text/html"));
        assert_eq!(args.to_schema.as_deref(), Some("html.rng"));
        assert_eq!(args.out, Some(PathBuf::from("view.html")));
    }

    #[test]
    fn transform_config_parses() {
        let cli = try_parse(&[
            "transform",
            "--config",
            "graph.cem",
            "--config-content-type",
            "text/cem-ml",
            "--config-schema",
            "https://cem.dev/ns/cli/transform-config/1",
        ])
        .unwrap();

        let Command::Transform(args) = cli.command else {
            panic!("expected transform command");
        };
        assert_eq!(args.data, None);
        assert_eq!(args.template, None);
        assert_eq!(args.config, Some(PathBuf::from("graph.cem")));
        assert_eq!(args.config_content_type.as_deref(), Some("text/cem-ml"));
        assert_eq!(
            args.config_schema.as_deref(),
            Some("https://cem.dev/ns/cli/transform-config/1")
        );
    }

    #[test]
    fn transform_requires_template() {
        assert!(try_parse(&[
            "transform",
            "data.xml",
            "--data-content-type",
            "application/xml",
            "--to-content-type",
            "text/html",
            "--out",
            "view.html",
        ])
        .is_err());
    }

    #[test]
    fn commands_accept_namespace_context_options() {
        let cli = try_parse(&[
            "validate",
            "--default-namespace",
            "urn:default",
            "--namespace",
            "html=https://www.w3.org/1999/xhtml",
            "--namespace",
            "svg=http://www.w3.org/2000/svg",
            "in.cem",
        ])
        .unwrap();

        let Command::Validate(args) = cli.command else {
            panic!("expected validate command");
        };
        assert_eq!(
            args.context.default_namespace.as_deref(),
            Some("urn:default")
        );
        assert_eq!(args.context.namespaces.len(), 2);
        assert_eq!(args.context.namespaces[0].prefix, "html");
        assert_eq!(
            args.context.namespaces[0].uri,
            "https://www.w3.org/1999/xhtml"
        );
        assert!(try_parse(&["validate", "--namespace", "=urn:default", "in.cem"]).is_err());
        assert!(try_parse(&["validate", "--namespace", "html", "in.cem"]).is_err());
    }

    #[test]
    fn commands_accept_scope_context_options() {
        let cli = try_parse(&[
            "validate",
            "--module-map",
            "cem.modules.json",
            "--version-pin",
            "cem-ml=1",
            "--scope-policy",
            "deterministic",
            "--scope-budget",
            "parseMs=5",
            "--scope-budget",
            "validateMs=7",
            "in.cem",
        ])
        .unwrap();

        let Command::Validate(args) = cli.command else {
            panic!("expected validate command");
        };
        assert_eq!(args.context.module_map.as_deref(), Some("cem.modules.json"));
        assert_eq!(args.context.version_pins[0].key, "cem-ml");
        assert_eq!(args.context.version_pins[0].value, "1");
        assert_eq!(args.context.scope_policy.as_deref(), Some("deterministic"));
        assert_eq!(args.context.scope_budgets.len(), 2);
        assert_eq!(args.context.scope_budgets[0].key, "parseMs");
        assert_eq!(args.context.scope_budgets[0].value, "5");
        assert!(try_parse(&["validate", "--version-pin", "=1", "in.cem"]).is_err());
        assert!(try_parse(&["validate", "--scope-budget", "parseMs", "in.cem"]).is_err());
    }

    #[test]
    fn commands_accept_resolver_map_context_options() {
        let cli = try_parse(&[
            "validate",
            "--resolver-read-map",
            "cem+vfs://workspace=/tmp/cem-vfs",
            "--resolver-write-map",
            "https://example.test/out=/tmp/cem-out",
            "in.cem",
        ])
        .unwrap();

        let Command::Validate(args) = cli.command else {
            panic!("expected validate command");
        };
        assert_eq!(args.context.resolver_read_maps.len(), 1);
        assert_eq!(
            args.context.resolver_read_maps[0].uri_prefix,
            "cem+vfs://workspace"
        );
        assert_eq!(
            args.context.resolver_read_maps[0].local_root,
            PathBuf::from("/tmp/cem-vfs")
        );
        assert_eq!(args.context.resolver_write_maps.len(), 1);
        assert_eq!(
            args.context.resolver_write_maps[0].uri_prefix,
            "https://example.test/out"
        );
        assert!(try_parse(&[
            "validate",
            "--resolver-read-map",
            "relative=/tmp/cem-vfs",
            "in.cem"
        ])
        .is_err());
        assert!(try_parse(&[
            "validate",
            "--resolver-write-map",
            "cem+vfs://workspace",
            "in.cem"
        ])
        .is_err());
    }

    #[test]
    fn commands_accept_run_spec_options() {
        try_parse(&[
            "validate",
            "--input-spec",
            "uri=in.cem,contentType=application/cem+xml,schema=core",
        ])
        .unwrap();
        try_parse(&[
            "convert",
            "--input-spec",
            "uri=in.html,contentType=text/html",
            "--output-spec",
            "dest=out.cem,contentType=application/cem+xml",
        ])
        .unwrap();
        try_parse(&[
            "validate",
            "--config",
            "cem-run.json",
            "--config-content-type",
            "application/json",
            "--config-schema",
            "https://cem.dev/ns/cli/run-config/1",
        ])
        .unwrap();
    }

    #[test]
    fn quiet_and_verbose_conflict() {
        assert!(try_parse(&["--quiet", "--verbose", "version"]).is_err());
    }

    #[test]
    fn fail_level_enum_values() {
        for lvl in ["parse", "validate", "strict"] {
            try_parse(&["validate", "--fail-level", lvl, "in.cem"]).expect(lvl);
        }
        assert!(try_parse(&["validate", "--fail-level", "warn", "in.cem"]).is_err());
    }

    #[test]
    fn iterations_must_be_at_least_one() {
        try_parse(&["bench", "--iterations", "1", "in.cem"]).unwrap();
        assert!(try_parse(&["bench", "--iterations", "0", "in.cem"]).is_err());
    }

    #[test]
    fn budget_ms_must_be_at_least_one() {
        try_parse(&["bench", "--budget-ms", "1", "in.cem"]).unwrap();
        assert!(try_parse(&["bench", "--budget-ms", "0", "in.cem"]).is_err());
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        assert!(try_parse(&["bogus"]).is_err());
    }

    #[test]
    fn fixture_subcommands_parse() {
        try_parse(&["fixture", "validate"]).unwrap();
        try_parse(&["fixture", "roundtrip"]).unwrap();
        try_parse(&["fixture", "validate", "a.cem", "b.cem"]).unwrap();
    }

    #[test]
    fn reserved_subcommands_parse() {
        try_parse(&["transform", "data.xml", "--template", "view.xsl"]).unwrap();
        try_parse(&["schema", "emit"]).unwrap();
        try_parse(&["schema", "sample"]).unwrap();
        try_parse(&["schema", "replace"]).unwrap();
        try_parse(&["plugin", "list"]).unwrap();
        try_parse(&["plugin", "inspect"]).unwrap();
        try_parse(&["plugin", "run"]).unwrap();
    }

    #[test]
    fn validate_requires_input() {
        let parsed = try_parse(&["validate"]).unwrap();
        match parsed.command {
            Command::Validate(args) => {
                assert!(args.inputs.is_empty() && args.run.input_specs.is_empty())
            }
            _ => panic!("expected validate"),
        }
    }

    #[test]
    fn fixture_validate_allows_empty_inputs() {
        try_parse(&["fixture", "validate"]).unwrap();
    }
}
