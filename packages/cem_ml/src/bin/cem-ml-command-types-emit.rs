use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match output_directory().and_then(|output| {
        cem_ml::typescript::emit_command_service_types_v1(&output)?;
        println!(
            "Generated command-service TypeScript declarations in {}",
            output.display()
        );
        Ok(())
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cem-ml-command-types-emit: {error}");
            ExitCode::FAILURE
        }
    }
}

fn output_directory() -> Result<PathBuf, String> {
    let mut arguments = std::env::args_os().skip(1);
    match (
        arguments.next().as_deref(),
        arguments.next(),
        arguments.next(),
    ) {
        (Some(flag), Some(path), None) if flag == "--out" => Ok(PathBuf::from(path)),
        _ => Err("usage: cem-ml-command-types-emit --out <directory>".to_owned()),
    }
}
