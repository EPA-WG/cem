mod graphql;
pub use graphql::{compile_graphql_contracts, MAX_GRAPHQL_SOURCE_BYTES};

use crate::{
    application_requirements::{
        registry_rust_source, reviewed_function, reviewed_pipeline, selected_execution_units,
        template_requirements, ApplicationRequirements, FunctionRequirement, PipelineOperation,
        TemplateRequirements,
    },
    declaration_closure::references,
    render::CompileTemplateOptions,
    selected_registry::{NativeExecutionUnit, SelectedBuiltin},
    template_artifact::{
        compile_template_artifact_with_compiled_styles, CompiledTemplateArtifact,
        PinnedTemplateArtifactContext, TemplateArtifactSourceMapMode,
    },
};
use cem_ml::{
    content_cache::ContentHash,
    validation::json::graphql::{GraphqlContracts, OperationKind},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_ENTRY_SOURCE_BYTES: usize = 1024 * 1024;

pub struct BuiltApplicationEntry {
    template: CompiledTemplateArtifact,
    requirements: EntryRequirements,
}

impl BuiltApplicationEntry {
    pub fn template(&self) -> &CompiledTemplateArtifact {
        &self.template
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryRequirements {
    source_uri: String,
    template_format: String,
    template: PinnedTemplateArtifactContext,
    component_roots: BTreeSet<String>,
    inventory: TemplateRequirements,
}

pub fn compile_application_entry(
    source: &str,
    source_uri: &str,
    host_bindings: Vec<String>,
    mode: TemplateArtifactSourceMapMode,
) -> Result<BuiltApplicationEntry, String> {
    if source.len() > MAX_ENTRY_SOURCE_BYTES || source_uri.len() > 4096 {
        return Err("application entry source exceeds its build limit".into());
    }
    if host_bindings.len() > 64
        || host_bindings
            .iter()
            .any(|name| name.is_empty() || name.len() > 256)
        || host_bindings.iter().collect::<BTreeSet<_>>().len() != host_bindings.len()
    {
        return Err("invalid or duplicate application entry binding".into());
    }
    let uri = url::Url::parse(source_uri).map_err(|e| e.to_string())?;
    if uri.cannot_be_a_base() || uri.fragment().is_some() {
        return Err("application entry needs a hierarchical source URI without a fragment".into());
    }
    let template = compile_template_artifact_with_compiled_styles(
        source,
        &CompileTemplateOptions {
            source_uri: Some(uri.to_string()),
            host_bindings,
            ..Default::default()
        },
        mode,
    )
    .map_err(|e| e.to_string())?;
    let pins = PinnedTemplateArtifactContext {
        content_hash: template.content_hash.header_value(),
        source_hash: template.identity.source_hash.header_value(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        host_bindings: template.identity.host_bindings.clone(),
        source_map_mode: mode,
    };
    let loaded = template.reload_pinned(&pins).map_err(|e| e.to_string())?;
    let mut component_roots = BTreeSet::new();
    references(&loaded.nodes, &mut component_roots)?;
    let inventory = template_requirements(&loaded, uri.as_str(), &pins.source_hash, false)?;
    let requirements = EntryRequirements {
        source_uri: uri.into(),
        template_format: template.identity.artifact_version.clone(),
        template: pins,
        component_roots,
        inventory,
    };
    Ok(BuiltApplicationEntry {
        template,
        requirements,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DispatchMapping<'a, T> {
    demand: &'a T,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_builtin: Option<SelectedBuiltin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gap: Option<String>,
}

impl<'a, T> DispatchMapping<'a, T> {
    fn new(demand: &'a T, selection: Result<Option<SelectedBuiltin>, String>) -> Self {
        let (selected_builtin, gap) = match selection {
            Ok(selection) => (selection, None),
            Err(gap) => (None, Some(gap)),
        };
        Self {
            demand,
            selected_builtin,
            gap,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DispatchMappings<'a> {
    functions: Vec<DispatchMapping<'a, FunctionRequirement>>,
    pipeline: Vec<DispatchMapping<'a, PipelineOperation>>,
}

impl<'a> DispatchMappings<'a> {
    fn new(
        functions: &'a BTreeSet<FunctionRequirement>,
        pipeline: &'a BTreeSet<PipelineOperation>,
    ) -> Self {
        Self {
            functions: functions
                .iter()
                .map(|f| DispatchMapping::new(f, reviewed_function(f).map(Some)))
                .collect(),
            pipeline: pipeline
                .iter()
                .map(|p| DispatchMapping::new(p, reviewed_pipeline(p)))
                .collect(),
        }
    }

    fn selected(&self) -> impl Iterator<Item = SelectedBuiltin> + '_ {
        self.functions
            .iter()
            .filter_map(|f| f.selected_builtin)
            .chain(self.pipeline.iter().filter_map(|p| p.selected_builtin))
    }

    fn has_gaps(&self) -> bool {
        self.functions.iter().any(|f| f.gap.is_some())
            || self.pipeline.iter().any(|p| p.gap.is_some())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryDependency {
    module: &'static str,
    name: &'static str,
    arity: usize,
    execution_units: BTreeSet<NativeExecutionUnit>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphqlDependency<'a> {
    schema_hash: &'a str,
    document_hash: &'a str,
    guard_hash: String,
    operations: BTreeMap<&'a str, OperationKind>,
}

pub struct BuiltDependencyManifest {
    pub bytes: Vec<u8>,
    pub content_hash: String,
    pub static_inventory_complete: bool,
    /// A standalone compiled JSON entry whose own dispatch and source are closed.
    pub selected_runtime_source: Option<String>,
}

/// This is build inventory. Independent runtime owners and link evidence still
/// have to be established for the application that consumes it.
pub fn build_dependency_manifest(
    requirements: &ApplicationRequirements,
    entry: Option<&BuiltApplicationEntry>,
    graphql: &GraphqlContracts,
) -> Result<BuiltDependencyManifest, String> {
    let (_, requirements_hash) = requirements.pinned_json()?;
    let entry = entry.map(|built| &built.requirements);
    if entry.is_some_and(|entry| !entry.component_roots.is_subset(&requirements.roots)) {
        return Err("application entry uses a component absent from the declared roots".into());
    }
    let (_, guard_hash) = graphql.pinned_bytes()?;
    let declarations: BTreeMap<_, _> = requirements
        .declarations
        .iter()
        .map(|(tag, d)| {
            (
                tag.as_str(),
                DispatchMappings::new(&d.functions, &d.pipeline_operations),
            )
        })
        .collect();
    let entry_dispatch = entry
        .map(|e| DispatchMappings::new(&e.inventory.functions, &e.inventory.pipeline_operations));
    let mappings = || declarations.values().chain(entry_dispatch.iter());
    let selected: BTreeSet<_> = mappings().flat_map(DispatchMappings::selected).collect();
    let execution_units = selected_execution_units(&selected);
    let registry_entries: BTreeMap<_, _> = selected
        .iter()
        .map(|&builtin| {
            let (module, name, arity) = builtin.key();
            (
                builtin,
                RegistryDependency {
                    module,
                    name,
                    arity,
                    execution_units: builtin.execution_units(),
                },
            )
        })
        .collect();
    let static_inventory_complete = requirements.require_static_inventory().is_ok()
        && entry.is_none_or(|e| e.inventory.unresolved.is_empty())
        && !mappings().any(DispatchMappings::has_gaps);

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Manifest<'a> {
        contract: &'static str,
        requirements_hash: String,
        requirements: &'a ApplicationRequirements,
        entry: Option<&'a EntryRequirements>,
        entry_dispatch: Option<DispatchMappings<'a>>,
        declarations: BTreeMap<&'a str, DispatchMappings<'a>>,
        selected_registry_entries: BTreeMap<SelectedBuiltin, RegistryDependency>,
        execution_units: BTreeSet<NativeExecutionUnit>,
        source_paths: BTreeSet<&'static str>,
        graphql: GraphqlDependency<'a>,
        static_inventory_complete: bool,
        native_link_closure_verified: bool,
    }
    let source_paths = execution_units
        .iter()
        .flat_map(|unit| unit.source_paths().iter().copied())
        .collect();
    let manifest = Manifest {
        contract: "cem-application-build/1",
        requirements_hash,
        requirements,
        entry,
        entry_dispatch,
        declarations,
        selected_registry_entries: registry_entries,
        execution_units,
        source_paths,
        graphql: GraphqlDependency {
            schema_hash: graphql.schema_hash(),
            document_hash: graphql.document_hash(),
            guard_hash,
            operations: graphql
                .operations()
                .iter()
                .map(|(name, op)| (name.as_str(), op.kind))
                .collect(),
        },
        static_inventory_complete,
        native_link_closure_verified: false,
    };
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("application build manifest exceeds 2 MiB".into());
    }
    let content_hash = ContentHash::from_blake3(&bytes).header_value();
    let selected_runtime_source = entry
        .zip(manifest.entry_dispatch.as_ref())
        .filter(|(entry, dispatch)| {
            entry.template.host_bindings.len() == 1
                && entry.inventory.unresolved.is_empty()
                && entry.inventory.data_imports.is_empty()
                && !dispatch.has_gaps()
        })
        .map(|(entry, _)| {
            let entry_pins = serde_json::to_string(&entry.template).map_err(|e| e.to_string())?;
            let mut source = registry_rust_source(&selected, &content_hash);
            source.push_str(&format!(
                r#"
pub const ENTRY: &[u8] = include_bytes!("application-entry.bin");
pub const ENTRY_PINS: &str = {entry_pins:?};
pub const GUARD: &[u8] = include_bytes!("graphql-contracts.bin");
pub const GUARD_HASH: &str = {:?};
pub const OPERATIONS: &str = include_str!("graphql-operations.graphql");
"#,
                manifest.graphql.guard_hash
            ));
            Ok::<_, String>(source)
        })
        .transpose()?;
    Ok(BuiltDependencyManifest {
        bytes,
        content_hash,
        static_inventory_complete,
        selected_runtime_source,
    })
}
