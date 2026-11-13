//! Native host adapter for the shared immutable-address filesystem plan.
use crate::cli::{Cli, Command, ContextOptions, RunOptions};
use cem_ml::{
    resolver::{
        PreparedResolverWrite, PublicationCapability, ResolveDirection, ResolveListRequest,
        ResolveRequest, ResolvedListEntry, ResolvedRead, ResolvedWrite, ResolverDiagnostic,
        ResolverRegistry, ResourceResolver,
    },
    studio_project::{
        parse_studio_project, STUDIO_PROJECT_CEM_CONTENT_TYPE, STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    },
    studio_project_filesystem::{plan_project_filesystem, ProjectFilesystemPlan},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct ProjectFilesystem {
    pub root: PathBuf,
    pub manifest: PathBuf,
    pub plan: ProjectFilesystemPlan,
}

impl ProjectFilesystem {
    /// Bind effective run-config addresses; the original configuration bytes and
    /// declared references stay untouched, as in the common invocation lowerer.
    pub(crate) fn bind_config(
        &self,
        config: &mut cem_ml::run_config::RunConfig,
        config_uri: Option<&str>,
    ) -> Result<(), String> {
        if config.resolvers.iter().any(|map| {
            map.uri_prefix.starts_with("studio:") || map.uri_prefix.starts_with("cem-studio:")
        }) {
            return Err(
                "cem.studio.filesystem.mapping_conflict: project mappings cannot be overridden"
                    .into(),
            );
        }
        for input in config.inputs.iter_mut().chain(&mut config.schema_packages) {
            input.uri = self.plan.read_uri(&input.uri)?;
            if let Some(module_map) = &mut input.root_scope.module_map {
                *module_map = self.config_reference(module_map, config_uri);
            }
        }
        for output in &mut config.outputs {
            if let Some(input) = &mut output.input_ref {
                *input = self.plan.read_uri(input)?;
            }
            if let Some(destination) = &mut output.destination {
                *destination = self.output_address(destination, config_uri)?;
            }
            if let Some(module_map) = &mut output.root_scope.module_map {
                *module_map = self.config_reference(module_map, config_uri);
            }
        }
        Ok(())
    }

    fn config_reference(&self, value: &str, config_uri: Option<&str>) -> String {
        if cem_ml::resolver::uri_scheme(value).is_some() || Path::new(value).is_absolute() {
            value.into()
        } else {
            cem_ml::transform_graph_request::resolve_transform_graph_reference(
                config_uri.unwrap_or(&self.plan.root_uri),
                value,
            )
        }
    }

    fn output_address(&self, value: &str, config_uri: Option<&str>) -> Result<String, String> {
        let mut path = Some(PathBuf::from(self.config_reference(value, config_uri)));
        self.output(&mut path)?;
        Ok(path.unwrap().to_string_lossy().into_owned())
    }

    fn map_path(&self, path: &mut PathBuf) -> Result<(), String> {
        *path = self.plan.read_uri(&path.to_string_lossy())?.into();
        Ok(())
    }
    fn map_optional(&self, path: &mut Option<PathBuf>) -> Result<(), String> {
        if let Some(path) = path {
            self.map_path(path)?;
        }
        Ok(())
    }
    fn input(
        &self,
        path: &mut Option<PathBuf>,
        context: &mut ContextOptions,
    ) -> Result<(), String> {
        self.map_optional(path)?;
        if let Some(file) = path
            .as_ref()
            .and_then(|path| self.plan.file(&path.to_string_lossy()))
        {
            context
                .content_type
                .get_or_insert_with(|| file.content_type.clone());
            if context.schema.is_none() {
                context.schema = file.schema.clone();
            }
        }
        Ok(())
    }
    fn context(&self, context: &mut ContextOptions) -> Result<(), String> {
        if context
            .resolver_read_maps
            .iter()
            .chain(&context.resolver_write_maps)
            .any(|map| {
                map.uri_prefix.starts_with("studio:") || map.uri_prefix.starts_with("cem-studio:")
            })
        {
            return Err(
                "cem.studio.filesystem.mapping_conflict: project mappings cannot be overridden"
                    .into(),
            );
        }
        if let Some(uri) = &mut context.module_map {
            *uri = self.plan.read_uri(uri)?;
        }
        for uri in &mut context.schema_packages {
            *uri = self.plan.read_uri(uri)?;
        }
        context.project_filesystem = Some(self.clone());
        Ok(())
    }
    fn run(&self, run: &mut RunOptions) -> Result<(), String> {
        self.map_optional(&mut run.config)?;
        if let Some(file) = run
            .config
            .as_ref()
            .and_then(|path| self.plan.file(&path.to_string_lossy()))
        {
            run.config_content_type
                .get_or_insert_with(|| file.content_type.clone());
            if run.config_schema.is_none() {
                run.config_schema = file.schema.clone();
            }
        }
        for record in &mut run.input_specs {
            *record =
                cem_ml::run_config::map_spec_record_addresses(record, &["uri", "path"], |uri| {
                    self.plan.read_uri(uri)
                })?;
        }
        for record in &mut run.output_specs {
            *record = cem_ml::run_config::map_spec_record_addresses(
                record,
                &["input", "input-ref", "inputRef"],
                |uri| self.plan.read_uri(uri),
            )?;
        }
        for record in &mut run.output_specs {
            *record = cem_ml::run_config::map_spec_record_addresses(
                record,
                &["dest", "destination"],
                |uri| self.output_address(uri, run.config.as_ref().and_then(|path| path.to_str())),
            )?;
        }
        Ok(())
    }

    fn reports(&self, report: &mut crate::cli::ReportOptions) -> Result<(), String> {
        self.output(&mut report.report)?;
        self.output(&mut report.report_json)?;
        self.output(&mut report.report_md)
    }

    fn destination(&self, uri: &str) -> Result<PathBuf, String> {
        let path = self.root.join(self.plan.output_path(uri)?);
        let mut ancestor = path.as_path();
        while !ancestor.exists() {
            ancestor = ancestor
                .parent()
                .ok_or("cem.studio.filesystem.output_path_invalid")?;
        }
        let ancestor = ancestor.canonicalize().map_err(|error| error.to_string())?;
        if !ancestor.starts_with(&self.root) {
            return Err("cem.studio.filesystem.path_escape".into());
        }
        if let Ok(canonical) = path.canonicalize() {
            if canonical == self.manifest
                || self.plan.files.iter().any(|file| {
                    self.root.join(&file.path).canonicalize().ok().as_ref() == Some(&canonical)
                })
            {
                return Err("cem.studio.filesystem.output_source_conflict".into());
            }
        }
        Ok(path)
    }

    fn output(&self, path: &mut Option<PathBuf>) -> Result<(), String> {
        if let Some(path) = path {
            let raw = path.to_string_lossy();
            if raw != "-"
                && !path.is_absolute()
                && (cem_ml::resolver::uri_scheme(&raw).is_none() || raw.starts_with("studio:"))
            {
                *path = self.plan.output_uri(&raw)?.into();
            } else if path.exists() {
                let canonical = path.canonicalize().map_err(|error| error.to_string())?;
                if canonical == self.manifest
                    || self.plan.files.iter().any(|file| {
                        self.root.join(&file.path).canonicalize().ok().as_ref() == Some(&canonical)
                    })
                {
                    return Err("cem.studio.filesystem.output_source_conflict".into());
                }
            }
        }
        Ok(())
    }
}

pub fn prepare(cli: &mut Cli) -> Result<(), String> {
    let Some(path) = &cli.resolver_project else {
        return Ok(());
    };
    let path = path
        .canonicalize()
        .map_err(|error| format!("cem.studio.filesystem.manifest: {error}"))?;
    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let content_type = if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        STUDIO_PROJECT_JSON_CONTENT_TYPE
    } else {
        STUDIO_PROJECT_CEM_CONTENT_TYPE
    };
    let project = parse_studio_project(&bytes, content_type, STUDIO_PROJECT_SCHEMA_URI)
        .map_err(|error| error.to_string())?;
    let filesystem = ProjectFilesystem {
        root: path.parent().unwrap_or(Path::new(".")).to_owned(),
        manifest: path,
        plan: plan_project_filesystem(&project)?,
    };
    filesystem.output(&mut cli.observe_events)?;
    match &mut cli.command {
        Command::Parse(a) => { filesystem.input(&mut a.input, &mut a.context)?; filesystem.run(&mut a.run)?; filesystem.output(&mut a.out)?; filesystem.reports(&mut a.report)?; filesystem.context(&mut a.context)?; }
        Command::Inspect(a) => { filesystem.input(&mut a.input, &mut a.context)?; filesystem.run(&mut a.run)?; filesystem.output(&mut a.out)?; filesystem.reports(&mut a.report)?; filesystem.context(&mut a.context)?; }
        Command::Convert(a) => { filesystem.input(&mut a.input, &mut a.context)?; filesystem.run(&mut a.run)?; filesystem.output(&mut a.out)?; filesystem.output(&mut a.artifact_json)?; filesystem.reports(&mut a.report)?; filesystem.context(&mut a.context)?; }
        Command::Trace(a) => { filesystem.input(&mut a.input, &mut a.context)?; filesystem.run(&mut a.run)?; filesystem.output(&mut a.out)?; filesystem.context(&mut a.context)?; }
        Command::Validate(a) => { for path in &mut a.inputs { filesystem.map_path(path)?; } filesystem.run(&mut a.run)?; filesystem.reports(&mut a.report)?; filesystem.context(&mut a.context)?; }
        Command::Check(a) => { for path in &mut a.inputs { filesystem.map_path(path)?; } filesystem.run(&mut a.run)?; filesystem.reports(&mut a.report)?; filesystem.context(&mut a.context)?; }
        Command::Bench(a) => { for path in &mut a.inputs { filesystem.map_path(path)?; } filesystem.run(&mut a.run)?; filesystem.context(&mut a.context)?; }
        Command::Transform(a) => {
            filesystem.map_optional(&mut a.config)?; filesystem.map_optional(&mut a.data)?; filesystem.map_optional(&mut a.template)?;
            if let Some(file) = a.config.as_ref().and_then(|path| filesystem.plan.file(&path.to_string_lossy())) {
                a.config_content_type.get_or_insert_with(|| file.content_type.clone());
                if a.config_schema.is_none() { a.config_schema = file.schema.clone(); }
            }
            if let Some(file) = a.template.as_ref().and_then(|path| filesystem.plan.file(&path.to_string_lossy())) {
                a.template_content_type.get_or_insert_with(|| file.content_type.clone());
                if a.template_schema.is_none() { a.template_schema = file.schema.clone(); }
            }
            filesystem.context(&mut a.context)?;
            filesystem.output(&mut a.out)?; filesystem.reports(&mut a.report)?;
        }
        Command::Query(a) => { filesystem.map_path(&mut a.data)?; filesystem.map_optional(&mut a.query_file)?; filesystem.output(&mut a.out)?; filesystem.reports(&mut a.report)?; filesystem.context(&mut a.context)?; }
        _ => return Err("cem.studio.filesystem.command_unsupported: --resolver-project requires a document command".into()),
    }
    Ok(())
}

impl ResourceResolver for ProjectFilesystem {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        let uri = request
            .base_uri
            .as_ref()
            .map(|base| {
                cem_ml::transform_graph_request::resolve_transform_graph_reference(
                    base,
                    &request.uri,
                )
            })
            .unwrap_or_else(|| request.uri.clone());
        let fail = |message: String| ResolverDiagnostic::Io {
            uri: uri.clone(),
            message,
        };
        let file = self
            .plan
            .file(&uri)
            .ok_or_else(|| fail("cem.studio.filesystem.unresolved".into()))?;
        let path = self
            .root
            .join(&file.path)
            .canonicalize()
            .map_err(|error| fail(error.to_string()))?;
        if !path.starts_with(&self.root) {
            return Err(fail("cem.studio.filesystem.path_escape".into()));
        }
        let bytes = fs::read(path).map_err(|error| fail(error.to_string()))?;
        if cem_ml::command_service::sha256_hex(&bytes) != file.sha256 {
            return Err(fail("cem.studio.filesystem.source_changed: materialize the matching saved project revision".into()));
        }
        Ok(ResolvedRead {
            uri: file.uri.clone(),
            bytes,
            content_type: request
                .content_type_hint
                .clone()
                .or_else(|| Some(file.content_type.clone())),
        })
    }
    fn write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        self.prepare_write(request, bytes)?.commit()
    }
    fn publication_capability(&self) -> PublicationCapability {
        PublicationCapability::Transactional
    }
    fn prepare_write(
        &self,
        request: &ResolveRequest,
        bytes: &[u8],
    ) -> Result<Box<dyn PreparedResolverWrite>, ResolverDiagnostic> {
        let fail = |message: String| ResolverDiagnostic::Io {
            uri: request.uri.clone(),
            message,
        };
        let path = self.destination(&request.uri).map_err(fail)?;
        let local = crate::publication::PreparedLocalWrite::prepare(&path, bytes)
            .map_err(|error| fail(error.to_string()))?;
        Ok(Box::new(ProjectWrite {
            uri: request.uri.clone(),
            local,
        }))
    }
    fn list(
        &self,
        request: &ResolveListRequest,
    ) -> Result<Vec<ResolvedListEntry>, ResolverDiagnostic> {
        let uri = request
            .base_uri
            .as_ref()
            .map(|base| {
                cem_ml::transform_graph_request::resolve_transform_graph_reference(
                    base,
                    &request.uri,
                )
            })
            .unwrap_or_else(|| request.uri.clone());
        let mut entries = Vec::new();
        for file in &self.plan.files {
            if cem_ml::transform_graph_request::transform_graph_reference_matches(&uri, &file.uri)
                .map_err(|message| ResolverDiagnostic::Io {
                uri: uri.clone(),
                message,
            })? {
                entries.push(ResolvedListEntry {
                    uri: file.uri.clone(),
                    content_type: Some(file.content_type.clone()),
                });
            }
        }
        entries.sort_by(|a, b| a.uri.cmp(&b.uri));
        if let Some(limit) = request.max_entries {
            entries.truncate(limit.saturating_add(1));
        }
        Ok(entries)
    }
}

struct ProjectWrite {
    uri: String,
    local: crate::publication::PreparedLocalWrite,
}
impl PreparedResolverWrite for ProjectWrite {
    fn commit(&mut self) -> Result<ResolvedWrite, ResolverDiagnostic> {
        self.local
            .commit()
            .map(|_| ResolvedWrite {
                uri: self.uri.clone(),
            })
            .map_err(|error| ResolverDiagnostic::TransactionState {
                uri: self.uri.clone(),
                message: error.to_string(),
            })
    }
    fn rollback(&mut self) -> Result<(), ResolverDiagnostic> {
        self.local
            .rollback()
            .map_err(|error| ResolverDiagnostic::TransactionState {
                uri: self.uri.clone(),
                message: error.to_string(),
            })
    }
}

pub fn register(
    registry: &mut ResolverRegistry,
    filesystem: &ProjectFilesystem,
    purposes: &[cem_ml::resolver::ResolvePurpose],
    writes: &[cem_ml::resolver::ResolvePurpose],
) {
    for purpose in purposes {
        registry.register(
            "studio",
            *purpose,
            ResolveDirection::Read,
            filesystem.clone(),
        );
        registry.register(
            "studio",
            *purpose,
            ResolveDirection::List,
            filesystem.clone(),
        );
    }
    for purpose in writes {
        registry.register(
            "studio",
            *purpose,
            ResolveDirection::Write,
            filesystem.clone(),
        );
    }
}
