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

    #[arg(long, global = true, help = "Suppress success/info output (errors still surface)")]
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

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum FailLevel {
    Parse,
    Validate,
    Strict,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum OutputFormat {
    Text,
    Html,
    Json,
    Xml,
    Cem,
    Markdown,
    DomJson,
    Ast,
    Events,
    Tree,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum InputFormat {
    Cem,
    Html,
    Xml,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum LayerFormat {
    DomJson,
    Ast,
    Events,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum InspectView {
    Summary,
    Ast,
    Events,
    Diagnostics,
    SourceOffsets,
    Tree,
}

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum BenchProfile {
    Cpu,
    Memory,
}

#[derive(Args, Debug, Default, Clone)]
pub struct ContextOptions {
    #[arg(long, value_name = "URI-OR-FILE", help = "Schema URI or file to record on diagnostics/reports")]
    pub schema: Option<String>,

    #[arg(long, value_name = "TYPE", help = "Content type to record on diagnostics/reports")]
    pub content_type: Option<String>,

    #[arg(long, value_name = "URI", help = "Base URI for diagnostic/report URI normalization")]
    pub base_uri: Option<String>,
}

#[derive(Args, Debug, Default, Clone)]
pub struct ReportOptions {
    #[arg(long, value_name = "FILE-OR-DIR", help = "Write JSON report to file or default name in dir")]
    pub report_json: Option<PathBuf>,

    #[arg(long, value_name = "FILE-OR-DIR", help = "Write Markdown report to file or default name in dir")]
    pub report_md: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ParseArgs {
    #[arg(value_name = "INPUT", help = "Path to a CEM-ML/HTML/XML input")]
    pub input: PathBuf,

    #[arg(long, value_enum, default_value_t = OutputFormat::DomJson,
          help = "Output projection (dom-json|json|ast|events for parse)")]
    pub format: OutputFormat,

    #[arg(long = "from-format", value_enum, help = "Override input format detection")]
    pub from_format: Option<InputFormat>,

    #[arg(long, value_enum, default_value_t = FailLevel::Parse)]
    pub fail_level: FailLevel,

    #[arg(long, value_name = "FILE", help = "Write primary output to file (stdout if omitted)")]
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

    #[arg(long, value_enum, default_value_t = OutputFormat::Text,
          help = "Report projection (json|xml|cem|text|html|markdown)")]
    pub format: OutputFormat,

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

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

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
          help = "Output layer (dom-json|ast|events)")]
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

    #[arg(long, value_enum, default_value_t = OutputFormat::Json,
          help = "Trace projection (json|xml|cem|text|html)")]
    pub format: OutputFormat,

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

    #[arg(long, value_enum, default_value_t = OutputFormat::Text,
          help = "Bench report projection (text|json)")]
    pub format: OutputFormat,

    #[arg(long, value_name = "N", default_value_t = 1, help = "Number of iterations")]
    pub iterations: u32,

    #[arg(long = "budget-ms", value_name = "MS", help = "Fail when per-iteration wall time exceeds this budget")]
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
