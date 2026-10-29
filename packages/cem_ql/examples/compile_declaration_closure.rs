//! Build a static component closure from explicit entry/projected-content roots.
use cem_ql::{
    declaration_closure::{
        build_declaration_closure, build_declaration_closure_with_css, DeclarationBuildSource,
    },
    template_artifact::TemplateArtifactSourceMapMode,
};
use std::{env, fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() < 5 {
        return Err("usage: compile_declaration_closure <components-dir> <source-base-uri> <output-dir> <dev|prod> [--compiled-css] [--require-static-requirements] <root-tag>...".into());
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
    let compiled_css = args[4..].iter().any(|arg| arg == "--compiled-css");
    let roots: Vec<_> = args[4..]
        .iter()
        .filter(|arg| {
            !matches!(
                arg.as_str(),
                "--compiled-css" | "--require-static-requirements"
            )
        })
        .cloned()
        .collect();
    let inputs = Path::new(&args[0]).canonicalize()?;
    let declarations = |tag: &str| {
        let path = format!("{tag}/{tag}.xhtml");
        Ok(DeclarationBuildSource {
            source: fs::read_to_string(inputs.join(&path)).map_err(|e| e.to_string())?,
            uri: base.join(&path).map_err(|e| e.to_string())?.into(),
        })
    };
    let closure = if compiled_css {
        build_declaration_closure_with_css(&roots, mode, declarations, |request| {
            let url =
                url::Url::parse(&request.resolution.resolved_url).map_err(|e| e.to_string())?;
            let relative = base
                .make_relative(&url)
                .ok_or("CSS URL is outside the declared source root")?;
            if url.query().is_some()
                || url.fragment().is_some()
                || relative.starts_with('/')
                || relative.split('/').any(|part| part == "..")
            {
                return Err("CSS URL is outside the declared source root".into());
            }
            let file_base =
                url::Url::from_directory_path(&inputs).map_err(|_| "invalid build directory")?;
            let path = file_base
                .join(&relative)
                .map_err(|e| e.to_string())?
                .to_file_path()
                .map_err(|_| "CSS URL is not a local build asset")?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !path.starts_with(&inputs)
                || fs::metadata(&path).map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024
            {
                return Err("CSS build asset is outside the source root or oversized".into());
            }
            Ok(cem_ml::resolver::ResolvedRead {
                uri: url.into(),
                bytes: fs::read(path).map_err(|e| e.to_string())?,
                content_type: Some("text/css".into()),
            })
        })?
    } else {
        build_declaration_closure(&roots, mode, declarations)?
    };
    if args[4..]
        .iter()
        .any(|arg| arg == "--require-static-requirements")
    {
        closure.requirements.require_static_inventory()?;
    }
    let (requirements, requirements_hash) = closure.requirements.pinned_json()?;
    // Compile/validate the entire closure before writing any output.
    let out = Path::new(&args[2]);
    fs::create_dir_all(out)?;
    for (tag, built) in &closure.declarations {
        fs::write(out.join(format!("{tag}.bin")), &built.template.bytes)?;
        fs::write(out.join(format!("{tag}.json")), &built.metadata_json)?;
    }
    fs::write(out.join("declarations.html"), closure.loader_html)?;
    fs::write(out.join("requirements.json"), requirements)?;
    fs::write(
        out.join("requirements.hash"),
        format!("{requirements_hash}\n"),
    )?;
    fs::write(
        out.join("dependencies.json"),
        serde_json::to_vec_pretty(&closure.dependencies)?,
    )?;
    Ok(())
}
