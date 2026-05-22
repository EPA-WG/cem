use std::env;
use std::path::PathBuf;

use cem_ml::schema::compiler::{CompilerOptions, SchemaCompiler};
use cem_ml::schema::ir::CompiledSchema;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let out = parse_out_dir(env::args().skip(1))?;
    let schema = CompiledSchema::cem_core();
    let output = SchemaCompiler::emit_all(
        &schema,
        &CompilerOptions {
            emit_rust: true,
            ..Default::default()
        },
    )
    .map_err(|err| err.to_string())?;
    SchemaCompiler::write_to_disk(&output, &out).map_err(|err| err.to_string())?;
    println!(
        "wrote {} schema artifacts under {}",
        output.artifacts.len(),
        out.display()
    );
    Ok(())
}

fn parse_out_dir(args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let mut out = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let Some(value) = args.next() else {
                    return Err("--out requires a directory path".to_owned());
                };
                out = Some(PathBuf::from(value));
            }
            "-h" | "--help" => {
                return Err("usage: cem-ml-schema-emit --out <dir>".to_owned());
            }
            _ => return Err(format!("unexpected argument `{arg}`")),
        }
    }
    out.ok_or_else(|| "usage: cem-ml-schema-emit --out <dir>".to_owned())
}
