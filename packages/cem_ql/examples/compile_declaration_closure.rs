//! Build a static component closure from explicit entry/projected-content roots.
use cem_ql::{
    declaration_closure::{build_declaration_closure, DeclarationBuildSource},
    template_artifact::TemplateArtifactSourceMapMode,
};
use std::{env, fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() < 5 {
        return Err("usage: compile_declaration_closure <components-dir> <source-base-uri> <output-dir> <dev|prod> <root-tag>...".into());
    }
    let base = url::Url::parse(&args[1])?;
    if !base.path().ends_with('/') {
        return Err("source base URI must end in /".into());
    }
    let mode = match args[3].as_str() {
        "dev" => TemplateArtifactSourceMapMode::Dev,
        "prod" => TemplateArtifactSourceMapMode::Prod,
        _ => return Err("invalid source-map mode".into()),
    };
    let closure = build_declaration_closure(&args[4..], mode, |tag| {
        let path = format!("{tag}/{tag}.xhtml");
        Ok(DeclarationBuildSource {
            source: fs::read_to_string(Path::new(&args[0]).join(&path))
                .map_err(|e| e.to_string())?,
            uri: base.join(&path).map_err(|e| e.to_string())?.into(),
        })
    })?;
    // Compile/validate the entire closure before writing any output.
    let out = Path::new(&args[2]);
    fs::create_dir_all(out)?;
    for (tag, built) in &closure.declarations {
        fs::write(out.join(format!("{tag}.bin")), &built.template.bytes)?;
        fs::write(out.join(format!("{tag}.json")), &built.metadata_json)?;
    }
    fs::write(out.join("declarations.html"), closure.loader_html)?;
    fs::write(
        out.join("dependencies.json"),
        serde_json::to_vec_pretty(&closure.dependencies)?,
    )?;
    Ok(())
}
