mod cli;

use clap::Parser;
use cli::{Cli, Command, FixtureCmd, PluginCmd, SchemaCmd};
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_USAGE_OR_RESERVED: u8 = 2;

fn main() -> ExitCode {
    let args = Cli::parse();
    let quiet = args.quiet;

    match args.command {
        Command::Version => {
            println!("cem-ml {}", cem_ml::VERSION);
            ExitCode::from(EXIT_OK)
        }

        Command::Parse(_)
        | Command::Validate(_)
        | Command::Check(_)
        | Command::Inspect(_)
        | Command::Convert(_)
        | Command::Trace(_)
        | Command::Bench(_) => parser_backed_stub(quiet),

        Command::Fixture(FixtureCmd::Validate(_)) | Command::Fixture(FixtureCmd::Roundtrip(_)) => {
            parser_backed_stub(quiet)
        }

        Command::Transform => reserved("transform"),
        Command::Schema(SchemaCmd::Emit) => reserved("schema emit"),
        Command::Schema(SchemaCmd::Sample) => reserved("schema sample"),
        Command::Schema(SchemaCmd::Replace) => reserved("schema replace"),
        Command::Plugin(PluginCmd::List) => reserved("plugin list"),
        Command::Plugin(PluginCmd::Inspect) => reserved("plugin inspect"),
        Command::Plugin(PluginCmd::Run) => reserved("plugin run"),
    }
}

fn parser_backed_stub(quiet: bool) -> ExitCode {
    if !quiet {
        eprintln!(
            "cem-ml: parser engine not yet implemented; command surface is wired but produces no output."
        );
        eprintln!("        See docs/cem-ml-cli-plan.md Phase 11 for the parser-enabled milestone.");
    }
    ExitCode::from(EXIT_OK)
}

fn reserved(name: &str) -> ExitCode {
    eprintln!(
        "cem-ml: `{name}` is reserved until its subsystem plan exists (exit 2 per cem-ml-cli-contract.md)."
    );
    ExitCode::from(EXIT_USAGE_OR_RESERVED)
}
