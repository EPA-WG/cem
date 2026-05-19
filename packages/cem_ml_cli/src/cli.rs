use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cem-ml",
    bin_name = "cem-ml",
    about = "CEM parser/runtime CLI",
    long_about = "CEM parser/runtime CLI. See docs/cem-ml-cli-contract.md for the feature surface.",
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

    #[command(about = "Reserved: transform pipeline (not yet implemented)")]
    Transform,
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
    DomJson,
    Ast,
    Events,
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
        long,
        value_name = "URI",
        help = "Base URI for diagnostic/report URI normalization"
    )]
    pub base_uri: Option<String>,
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

#[derive(Args, Debug)]
pub struct ParseArgs {
    #[arg(value_name = "INPUT", help = "Path to a CEM-ML/HTML/XML input")]
    pub input: PathBuf,

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
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    #[arg(value_name = "INPUT", required = true, num_args = 1.., help = "One or more inputs")]
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
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    #[arg(value_name = "INPUT", required = true, num_args = 1..)]
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
    pub report: ReportOptions,
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    #[arg(long, value_enum, default_value_t = InspectView::Summary,
          help = "Which inspector view to render")]
    pub show: InspectView,

    #[arg(long = "from-format", value_enum)]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    #[command(flatten)]
    pub context: ContextOptions,
}

#[derive(Args, Debug)]
pub struct ConvertArgs {
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    #[arg(long = "from-format", value_enum, help = "Input syntax (cem|html|xml)")]
    pub from_format: Option<InputFormat>,

    #[arg(long = "to-format", value_enum, default_value_t = LayerFormat::DomJson,
          help = "Output layer (cem|dom-json|ast|events)")]
    pub to_format: LayerFormat,

    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    #[arg(long, help = "Preserve absolute source byte offsets in output")]
    pub preserve_source_offsets: bool,

    #[command(flatten)]
    pub context: ContextOptions,
}

#[derive(Args, Debug)]
pub struct TraceArgs {
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    #[arg(long, value_enum, default_value_t = TraceFormat::Json,
          help = "Trace projection (json|xml|cem|text|html)")]
    pub format: TraceFormat,

    #[arg(long = "from-format", value_enum)]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    #[command(flatten)]
    pub context: ContextOptions,
}

#[derive(Args, Debug)]
pub struct BenchArgs {
    #[arg(value_name = "INPUT", required = true, num_args = 1..)]
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
        for fmt in ["cem", "dom-json", "ast", "events"] {
            try_parse(&["convert", "--to-format", fmt, "in.cem"]).expect(fmt);
        }
        for fmt in ["json", "xml", "text", "html"] {
            assert!(
                try_parse(&["convert", "--to-format", fmt, "in.cem"]).is_err(),
                "rejected: {fmt}"
            );
        }
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
        try_parse(&["transform"]).unwrap();
        try_parse(&["schema", "emit"]).unwrap();
        try_parse(&["schema", "sample"]).unwrap();
        try_parse(&["schema", "replace"]).unwrap();
        try_parse(&["plugin", "list"]).unwrap();
        try_parse(&["plugin", "inspect"]).unwrap();
        try_parse(&["plugin", "run"]).unwrap();
    }

    #[test]
    fn validate_requires_input() {
        assert!(try_parse(&["validate"]).is_err());
    }

    #[test]
    fn fixture_validate_allows_empty_inputs() {
        try_parse(&["fixture", "validate"]).unwrap();
    }
}
