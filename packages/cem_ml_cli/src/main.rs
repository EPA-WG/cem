mod cli;
mod dispatch;

use cem_ml::real::RealCemMlEngine;
use clap::Parser;
use dispatch::{Outcome, Streams};
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let parsed = cli::Cli::parse();
    let quiet = parsed.quiet;
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    let mut streams = Streams {
        stdout: &mut out,
        stderr: &mut err,
        quiet,
    };
    let engine = RealCemMlEngine::new();
    let Outcome { exit_code } = dispatch::dispatch(&engine, parsed, &mut streams);
    ExitCode::from(exit_code)
}
