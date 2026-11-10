//! Build a static component closure from explicit entry/projected-content roots.
use cem_ql::{
    declaration_closure::{
        build_declaration_closure, build_declaration_closure_with_css, DeclarationBuildSource,
    },
    template_artifact::TemplateArtifactSourceMapMode,
};
use std::{env, fs, path::Path};
const APPLICATION_FLAGS: &[&str] = &[
    "--graphql-schema",
    "--graphql-operations",
    "--entry",
    "--entry-uri",
    "--entry-binding",
];

fn application_flag(arg: &str) -> bool {
    let flag = arg.split_once('=').map_or(arg, |(flag, _)| flag);
    APPLICATION_FLAGS.contains(&flag)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() < 5 {
        return Err("usage: compile_declaration_closure <components-dir> <source-base-uri> <output-dir> <dev|prod> [--compiled-css] [--require-static-requirements] [--selected-registry] [--graphql-schema=PATH --graphql-operations=PATH [--entry=PATH --entry-uri=URL --entry-binding=NAME ...]] <root-tag>...".into());
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
    let application_args: Vec<_> = args[4..]
        .iter()
        .filter(|arg| application_flag(arg))
        .collect();
    #[cfg(not(feature = "application-contract-build"))]
    if !application_args.is_empty() {
        return Err(
            "application contract flags require the application-contract-build feature".into(),
        );
    }
    #[cfg(feature = "application-contract-build")]
    let application = (!application_args.is_empty())
        .then(|| application::Inputs::read(&application_args, mode))
        .transpose()?;
    let compiled_css = args[4..].iter().any(|arg| arg == "--compiled-css");
    let roots: Vec<_> = args[4..]
        .iter()
        .filter(|arg| {
            !application_flag(arg)
                && !matches!(
                    arg.as_str(),
                    "--compiled-css" | "--require-static-requirements" | "--selected-registry"
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
    let selected = args[4..]
        .iter()
        .any(|arg| arg == "--selected-registry")
        .then(|| closure.requirements.selected_registry_plan())
        .transpose()?
        .map(|plan| {
            let (bytes, hash) = plan.pinned_json()?;
            Ok::<_, String>((bytes, hash, plan.rust_source()))
        })
        .transpose()?;
    #[cfg(feature = "application-contract-build")]
    let application = application
        .map(|inputs| {
            inputs.build(
                &closure.requirements,
                args[4..]
                    .iter()
                    .any(|arg| arg == "--require-static-requirements"),
                selected.is_some(),
            )
        })
        .transpose()?;
    // Compile/validate the entire closure before writing any output.
    let out = Path::new(&args[2]);
    fs::create_dir_all(out)?;
    // Publish the new link recipe last. A failed write must leave no old
    // recipe beside a partially replaced set of assets.
    match fs::remove_file(out.join("selected_application.rs")) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
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
    if let Some((bytes, hash, source)) = selected {
        fs::write(out.join("selected-registry.json"), bytes)?;
        fs::write(out.join("selected-registry.hash"), format!("{hash}\n"))?;
        fs::write(out.join("selected_registry.rs"), source)?;
    }
    fs::write(
        out.join("dependencies.json"),
        serde_json::to_vec_pretty(&closure.dependencies)?,
    )?;
    #[cfg(feature = "application-contract-build")]
    if let Some(application) = application {
        application.write(out)?;
    }
    Ok(())
}

#[cfg(feature = "application-contract-build")]
mod application {
    use cem_ql::{
        application_build::{
            build_dependency_manifest, compile_application_entry, compile_graphql_contracts,
            BuiltApplicationEntry, BuiltDependencyManifest, MAX_ENTRY_SOURCE_BYTES,
            MAX_GRAPHQL_SOURCE_BYTES,
        },
        application_requirements::ApplicationRequirements,
        template_artifact::TemplateArtifactSourceMapMode,
    };
    use std::{error::Error, fs, io::Read, path::Path};

    pub struct Inputs {
        schema: String,
        operations: String,
        entry: Option<BuiltApplicationEntry>,
    }

    pub struct Outputs {
        manifest: BuiltDependencyManifest,
        guard: (Vec<u8>, String),
        operations: String,
        entry: Option<BuiltApplicationEntry>,
    }

    impl Inputs {
        pub fn read(
            args: &[&String],
            mode: TemplateArtifactSourceMapMode,
        ) -> Result<Self, Box<dyn Error>> {
            let (mut schema, mut operations, mut entry, mut uri) = (None, None, None, None);
            let mut bindings = Vec::new();
            for arg in args {
                let (flag, value) = arg
                    .split_once('=')
                    .filter(|(_, value)| !value.is_empty())
                    .ok_or("application contract flags need a nonempty =VALUE")?;
                let slot = match flag {
                    "--graphql-schema" => &mut schema,
                    "--graphql-operations" => &mut operations,
                    "--entry" => &mut entry,
                    "--entry-uri" => &mut uri,
                    "--entry-binding" => {
                        bindings.push(value.to_owned());
                        continue;
                    }
                    _ => return Err("unknown application contract flag".into()),
                };
                if slot.replace(value).is_some() {
                    return Err(format!("duplicate {flag}").into());
                }
            }
            let schema = read(
                schema.ok_or("--graphql-schema is required")?,
                MAX_GRAPHQL_SOURCE_BYTES,
            )?;
            let operations = read(
                operations.ok_or("--graphql-operations is required")?,
                MAX_GRAPHQL_SOURCE_BYTES,
            )?;
            let entry = match (entry, uri) {
                (Some(path), Some(uri)) => Some(compile_application_entry(
                    &read(path, MAX_ENTRY_SOURCE_BYTES)?,
                    uri,
                    bindings,
                    mode,
                )?),
                (None, None) if bindings.is_empty() => None,
                _ => return Err("--entry and --entry-uri must be supplied together".into()),
            };
            Ok(Self {
                schema,
                operations,
                entry,
            })
        }

        pub fn build(
            self,
            requirements: &ApplicationRequirements,
            strict: bool,
            registry: bool,
        ) -> Result<Outputs, Box<dyn Error>> {
            if self.entry.is_some() && registry {
                return Err("--selected-registry is declaration-only. Use the application build manifest for an entry's registry inventory".into());
            }
            let graphql = compile_graphql_contracts(&self.schema, &self.operations)?;
            let manifest = build_dependency_manifest(requirements, self.entry.as_ref(), &graphql)?;
            if strict && !manifest.static_inventory_complete {
                return Err(
                    "application inventory includes unresolved entry or registry demands".into(),
                );
            }
            Ok(Outputs {
                manifest,
                guard: graphql.pinned_bytes()?,
                operations: self.operations,
                entry: self.entry,
            })
        }
    }

    impl Outputs {
        pub fn write(self, out: &Path) -> Result<(), Box<dyn Error>> {
            let (guard, guard_hash) = self.guard;
            fs::write(out.join("application-build.json"), self.manifest.bytes)?;
            fs::write(
                out.join("application-build.hash"),
                format!("{}\n", self.manifest.content_hash),
            )?;
            fs::write(out.join("graphql-contracts.bin"), guard)?;
            fs::write(
                out.join("graphql-contracts.hash"),
                format!("{guard_hash}\n"),
            )?;
            fs::write(out.join("graphql-operations.graphql"), self.operations)?;
            if let Some(entry) = self.entry {
                fs::write(out.join("application-entry.bin"), &entry.template().bytes)?;
            }
            if let Some(source) = self.manifest.selected_runtime_source {
                fs::write(out.join("selected_application.rs"), source)?;
            }
            Ok(())
        }
    }

    fn read(path: &str, max_bytes: usize) -> Result<String, Box<dyn Error>> {
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take((max_bytes + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > max_bytes {
            return Err(
                format!("application contract input exceeds {max_bytes} bytes: {path}").into(),
            );
        }
        Ok(String::from_utf8(bytes)?)
    }
}
