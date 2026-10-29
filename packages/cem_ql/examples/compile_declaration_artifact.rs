//! Native build tool; metadata and executable IR have independently pinned hashes.
use cem_ql::{
    declaration_artifact::build_declaration_artifact,
    template_artifact::TemplateArtifactSourceMapMode,
};
use std::{env, fs, path::Path};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if !(4..=5).contains(&args.len()) {
        fail("usage: compile_declaration_artifact <source.xhtml> <output-prefix> <source-uri> <dev|prod> [tag-alias]");
    }
    let mode = match args[3].as_str() {
        "dev" => TemplateArtifactSourceMapMode::Dev,
        "prod" => TemplateArtifactSourceMapMode::Prod,
        _ => fail("invalid source-map mode"),
    };
    let source = fs::read_to_string(&args[0]).unwrap_or_else(|error| fail(&error.to_string()));
    let built =
        build_declaration_artifact(&source, &args[2], mode, args.get(4).map(String::as_str))
            .unwrap_or_else(|error| fail(&error));
    if let Some(parent) = Path::new(&args[1]).parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| fail(&error.to_string()));
    }
    fs::write(format!("{}.bin", args[1]), built.template.bytes)
        .unwrap_or_else(|error| fail(&error.to_string()));
    fs::write(format!("{}.json", args[1]), built.metadata_json)
        .unwrap_or_else(|error| fail(&error.to_string()));
    fs::write(
        format!("{}.hash", args[1]),
        format!("{}\n", built.metadata_hash),
    )
    .unwrap_or_else(|error| fail(&error.to_string()));
}
fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}
