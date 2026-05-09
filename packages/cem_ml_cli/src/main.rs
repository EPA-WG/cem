use clap::Parser;

#[derive(Parser)]
#[command(name = "cem-ml", about = "CEM parser/runtime CLI", version = cem_ml::VERSION, disable_version_flag = true)]
struct Cli {
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    version: bool,
}

fn main() {
    Cli::parse();
}
