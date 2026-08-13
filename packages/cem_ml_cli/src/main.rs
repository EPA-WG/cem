use cem_ml::real::RealCemMlEngine;
use cem_ml_cli::{cli, dispatch};
use clap::Parser;
use dispatch::{Outcome, Streams};
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let parsed = cli::Cli::parse();
    #[cfg(feature = "debug-control")]
    if let cli::Command::Debug(arguments) = &parsed.command {
        return ExitCode::from(cem_ml_cli::debug_transport::run(arguments));
    }
    let quiet = parsed.quiet;
    let no_color = parsed.no_color;
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    let abort_signal = cem_ml::scheduler::AbortSignal::new();
    let signal_handler_abort = abort_signal.clone();
    if let Err(error) = ctrlc::set_handler(move || signal_handler_abort.abort()) {
        let _ = std::io::Write::write_fmt(
            &mut err,
            format_args!("cem-ml: cannot install signal handler: {error}\n"),
        );
        return ExitCode::from(dispatch::EXIT_INTERNAL);
    }
    let mut streams = Streams {
        stdout: &mut out,
        stderr: &mut err,
        quiet,
        no_color,
        abort_signal,
        #[cfg(feature = "debug-control")]
        operation_control: None,
    };
    let engine = RealCemMlEngine::new();
    let Outcome { exit_code } = dispatch::dispatch(&engine, parsed, &mut streams);
    ExitCode::from(exit_code)
}
