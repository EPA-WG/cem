use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os();
    let binary = arguments.next().unwrap_or_default();
    let output = arguments.next().map(PathBuf::from).ok_or_else(|| {
        format!(
            "usage: {} OUTPUT",
            PathBuf::from(binary)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        )
    })?;
    if arguments.next().is_some() {
        return Err("command-schema emitter accepts exactly one output path".into());
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let schema = cem_ml_cli::command_schema::shared_command_schema();
    let bytes = serde_json::to_vec_pretty(&schema)?;
    std::fs::write(&output, [bytes.as_slice(), b"\n"].concat())?;
    Ok(())
}
