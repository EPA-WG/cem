use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cem-ml", about = "CEM DOM parser", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show help information
    Help,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Help => {
            Cli::command().print_help().unwrap();
            println!();
        }
    }
}
