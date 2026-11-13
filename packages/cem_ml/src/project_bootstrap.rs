//! Shared project creation control boundary. All hosts receive the same editable
//! configuration and complete, validated archive; this producer performs no I/O.
//! Source documents remain exact UTF-8 bytes, imported by the ordinary engine
//! only when a generated command runs. Graphs use the existing native grammar.

use crate::cli_command::{
    parse_cli_command, CliCommandProjection, CliCommandResource, CLI_COMMAND_BINARY_NAME,
    CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_GRAMMAR_SCHEMA_VERSION, CLI_COMMAND_SCHEMA_URI,
    CLI_COMMAND_SCHEMA_VERSION,
};
use crate::command_service::sha256_hex;
use crate::engine::FormatIdentity;
use crate::studio_project::{
    validate_studio_project, StudioProject, StudioProjectEntry, StudioProjectEntryKind,
    StudioProjectProjection, StudioProjectResource, StudioProjectResourceRole,
    StudioProjectResourceSourceKind, STUDIO_PROJECT_SCHEMA_URI, STUDIO_PROJECT_SCHEMA_VERSION,
};
use crate::studio_project_filesystem::plan_project_filesystem;
use crate::transform_config::{
    parse_transform_graph_config, TransformGraphParseRequest, TRANSFORM_CONFIG_SCHEMA_URI,
};
use crate::transform_graph_request::resolve_transform_graph_reference;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
mod planning;
pub use planning::{plan_project, review_configuration, ProjectBootstrapAnswers};

pub const PROJECT_BOOTSTRAP_SCHEMA_URI: &str = "https://cem.dev/ns/project-bootstrap/1";
pub const PROJECT_BOOTSTRAP_SCHEMA_VERSION: u32 = 1;
/// This authoring API is deliberately bounded, including its JSON envelope.
pub const PROJECT_BOOTSTRAP_MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const MAX_RESOURCES: usize = 128;
const MAX_ENTRIES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectBootstrapPreset {
    MarkdownSite,
    TransformationPipeline,
    SimpleConversion,
    SchemaValidation,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBootstrapConfiguration {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub schema_version: u32,
    pub common_version: String,
    pub preset: ProjectBootstrapPreset,
    pub project_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: String,
    /// Reuse project topology, grouping and runnable entry contracts directly.
    pub entries: Vec<StudioProjectEntry>,
    pub resources: Vec<ProjectBootstrapResource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBootstrapResource {
    pub id: String,
    pub role: StudioProjectResourceRole,
    pub path: String,
    pub content_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
enum BootstrapRequest {
    Catalog {},
    #[serde(rename_all = "camelCase")]
    Preset {
        preset: ProjectBootstrapPreset,
        project_id: String,
        name: String,
        created_at: String,
    },
    Generate {
        configuration: ProjectBootstrapConfiguration,
    },
    #[serde(rename_all = "camelCase")]
    Plan {
        preset: ProjectBootstrapPreset, project_id: String, name: String, created_at: String,
        #[serde(default)]
        answers: ProjectBootstrapAnswers,
    },
    Review { configuration: ProjectBootstrapConfiguration },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBootstrapError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
}
impl std::fmt::Display for ProjectBootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ProjectBootstrapError {}
impl From<crate::studio_project::StudioProjectError> for ProjectBootstrapError {
    fn from(error: crate::studio_project::StudioProjectError) -> Self {
        Self {
            code: error.code.into(),
            message: error.message,
            field_path: error.field_path,
            diagnostics: vec![],
        }
    }
}
fn error(code: &str, message: impl Into<String>, field: Option<&str>) -> ProjectBootstrapError {
    ProjectBootstrapError {
        code: format!("cem.project_bootstrap.{code}"),
        message: message.into(),
        field_path: field.map(str::to_owned),
        diagnostics: vec![],
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBootstrapContent {
    pub resource_id: String,
    pub base64: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBootstrapBundle {
    pub archive_schema_version: u32,
    pub project: StudioProject,
    pub contents: Vec<ProjectBootstrapContent>,
}
#[derive(Debug, Serialize)]
pub struct ProjectBootstrapResult {
    pub bundle: ProjectBootstrapBundle,
    pub manifest: String,
}

/// Explicit control JSON ABI used by the CLI and the owned browser generation.
/// Errors never contain a partial archive. Publication is a host transaction.
pub fn project_bootstrap_json(request: &str) -> String {
    let result = if request.len() > PROJECT_BOOTSTRAP_MAX_REQUEST_BYTES {
        Err(error(
            "request_limit",
            "Bootstrap request exceeds 4 MiB",
            None,
        ))
    } else {
        serde_json::from_str::<BootstrapRequest>(request)
            .map_err(|e| error("request_invalid", e.to_string(), None))
            .and_then(|request| match request {
                BootstrapRequest::Catalog {} => Ok(json!({"presets": preset_catalog(), "configurationSchema": configuration_schema()})),
                BootstrapRequest::Preset {preset, project_id, name, created_at} => {
                    let config = preset_configuration(preset, project_id, name, created_at)?;
                    Ok(json!({"configuration":config}))
                }
                BootstrapRequest::Generate {configuration} => {
                    serde_json::to_value(generate_project(configuration)?)
                        .map_err(|e| error("serialization", e.to_string(), None))
                }
                BootstrapRequest::Plan {preset, project_id, name, created_at, answers} => {
                    plan_project(preset, project_id, name, created_at, answers)
                }
                BootstrapRequest::Review {configuration} => {
                    let review = review_configuration(&configuration)?;
                    Ok(json!({"configuration":configuration, "review":review}))
                }
            })
    };
    let mut value = result.unwrap_or_else(|e| json!({"error":e}));
    value["schemaVersion"] = PROJECT_BOOTSTRAP_SCHEMA_VERSION.into();
    value["commonVersion"] = crate::VERSION.into();
    serde_json::to_string(&value).expect("bootstrap control values are serializable")
}

pub fn generate_project(
    config: ProjectBootstrapConfiguration,
) -> Result<ProjectBootstrapResult, ProjectBootstrapError> {
    if config.schema != PROJECT_BOOTSTRAP_SCHEMA_URI
        || config.schema_version != PROJECT_BOOTSTRAP_SCHEMA_VERSION
    {
        return Err(error(
            "version_unsupported",
            "Unsupported project bootstrap schema identity or version",
            Some("$schema"),
        ));
    }
    if config.common_version != crate::VERSION {
        return Err(error(
            "common_version_mismatch",
            "Bootstrap configuration must match the native common version",
            Some("commonVersion"),
        ));
    }
    if config.resources.len() > MAX_RESOURCES
        || config.entries.len() > MAX_ENTRIES
        || serde_json::to_vec(&config)
            .map_or(true, |b| b.len() > PROJECT_BOOTSTRAP_MAX_REQUEST_BYTES)
    {
        return Err(error(
            "request_limit",
            "Bootstrap configuration exceeds the authoring limits",
            None,
        ));
    }
    let project = StudioProject {
        schema: STUDIO_PROJECT_SCHEMA_URI.into(),
        schema_version: STUDIO_PROJECT_SCHEMA_VERSION,
        root_uri: format!("studio://{}/", config.project_id),
        id: config.project_id,
        name: config.name,
        description: config.description,
        revision: 1,
        created_at: config.created_at.clone(),
        updated_at: config.created_at,
        entries: config.entries,
        resources: config
            .resources
            .iter()
            .map(|r| StudioProjectResource {
                id: r.id.clone(),
                role: r.role,
                source_kind: StudioProjectResourceSourceKind::ProjectFile,
                path: r.path.clone(),
                reference_path: Some(format!(".resources/{}", r.id)),
                url: None,
                content_type: r.content_type.clone(),
                schema: r.schema.clone(),
                revision: 1,
                sha256: sha256_hex(r.text.as_bytes()),
            })
            .collect(),
    };
    validate_studio_project(&project)?;
    let plan =
        plan_project_filesystem(&project).map_err(|e| error("filesystem_invalid", e, None))?;
    for (index, resource) in config.resources.iter().enumerate() {
        let field = format!("resources[{index}].text");
        if resource.role == StudioProjectResourceRole::RunConfig {
            // Command grammar is validated by the executing host's generated CLI
            // schema before publication, as for other authored command resources.
            let command = parse_cli_command(
                resource.text.as_bytes(),
                &resource.content_type,
                resource.schema.as_deref().unwrap_or(""),
            )
            .map_err(|e| error("command_invalid", e.to_string(), Some(&field)))?;
            if command.common_version != crate::VERSION {
                return Err(error(
                    "common_version_mismatch",
                    "Authored command must match the native common version",
                    Some(&field),
                ));
            }
        }
        if matches!(
            resource.role,
            StudioProjectResourceRole::Graph | StudioProjectResourceRole::TransformConfig
        ) {
            let uri = format!("{}.resources/{}", project.root_uri, resource.id);
            let response = parse_transform_graph_config(TransformGraphParseRequest {
                bytes: resource.text.as_bytes().to_vec(),
                base_uri: Some(uri.clone()),
                identity: FormatIdentity {
                    content_type: Some(resource.content_type.clone()),
                    schema: resource.schema.clone(),
                    ..FormatIdentity::default()
                },
            })
            .map_err(|e| error("graph_invalid", e.to_string(), Some(&field)))?;
            if let Some(diagnostic) = response
                .diagnostics
                .iter()
                .find(|d| d.severity.is_hard_violation())
            {
                let mut failure = error(
                    "graph_invalid",
                    format!("{}: {}", diagnostic.code, diagnostic.message),
                    Some(&field),
                );
                failure.diagnostics = response.diagnostics;
                return Err(failure);
            }
            for node in response.graph.nodes {
                // V1 is a self-contained local starter. Remote acquisition and
                // glob expansion require a separately reviewed resource policy.
                for reference in [
                    &node.src,
                    &node.source_map,
                    &node.authored_map,
                    &node.target_map,
                ]
                .into_iter()
                .flatten()
                {
                    let address = resolve_transform_graph_reference(&uri, reference);
                    if plan.file(&address).is_none() {
                        return Err(error(
                            "graph_resource_unresolved",
                            format!("Graph resource `{reference}` is not in this project"),
                            Some(&field),
                        ));
                    }
                }
                if let Some(out) = node.out {
                    let address = resolve_transform_graph_reference(&uri, &out);
                    plan.output_path(&address)
                        .map_err(|e| error("output_invalid", e, Some(&field)))?;
                }
            }
        }
    }
    // Nothing escapes until every identity, topology, path and graph is ready.
    let manifest = project.serialize(StudioProjectProjection::Cem)?;
    Ok(ProjectBootstrapResult {
        manifest,
        bundle: ProjectBootstrapBundle {
            archive_schema_version: 1,
            project,
            contents: config
                .resources
                .iter()
                .map(|r| ProjectBootstrapContent {
                    resource_id: r.id.clone(),
                    base64: base64::engine::general_purpose::STANDARD.encode(r.text.as_bytes()),
                })
                .collect(),
        },
    })
}

fn preset_catalog() -> Value {
    json!([
        {"id":"markdown-site","name":"Markdown site","description":"Build a Markdown home page as HTML; extend the shared graph for additional pages and layouts."},
        {"id":"transformation-pipeline","name":"Transformation pipeline","description":"Import XML and apply a native CEM template to publish HTML."},
        {"id":"simple-conversion","name":"Simple conversion","description":"Convert YAML to tabular CEM-ML through the shared native import boundary."},
        {"id":"schema-validation","name":"Schema validation","description":"Validate an XHTML document against the built-in XHTML schema."},
        {"id":"custom","name":"Minimal custom project","description":"Start with an empty project; add resources, subprojects and runnable entries before generation."}
    ])
}

pub fn preset_configuration(
    preset: ProjectBootstrapPreset,
    project_id: String,
    name: String,
    created_at: String,
) -> Result<ProjectBootstrapConfiguration, ProjectBootstrapError> {
    let mut config = ProjectBootstrapConfiguration {
        schema: PROJECT_BOOTSTRAP_SCHEMA_URI.into(),
        schema_version: PROJECT_BOOTSTRAP_SCHEMA_VERSION,
        common_version: crate::VERSION.into(),
        preset,
        project_id,
        name,
        description: None,
        created_at,
        entries: vec![],
        resources: vec![],
    };
    use StudioProjectResourceRole as Role;
    let (kind, title, argv, artifact) = match preset {
        ProjectBootstrapPreset::MarkdownSite => {
            config.resources.push(resource(
                "page",
                Role::Data,
                "content/index.md",
                "text/markdown",
                "https://cem.dev/ns/data/markdown/1",
                "# Welcome\n\nYour CEM site starts here.\n",
            ));
            config.resources.push(resource("pipeline", Role::Graph, "pipelines/site.cem", "application/cem", TRANSFORM_CONFIG_SCHEMA_URI,
                "@doc cem-ml 1\n{run | {import @id=source @src=page @content-type=text/markdown @schema=https://cem.dev/ns/data/markdown/1 | {convert @id=html @content-type=text/html @schema=https://cem.dev/ns/data/html/1 | {export @id=page @out=../generated/index.html @content-type=text/html @schema=https://cem.dev/ns/data/html/1}}}}\n"));
            (
                StudioProjectEntryKind::TransformationGraph,
                "Build site",
                vec!["transform", "--config", ".resources/pipeline"],
                Some("generated/index.html"),
            )
        }
        ProjectBootstrapPreset::TransformationPipeline => {
            config.resources.push(resource(
                "source",
                Role::Data,
                "data/source.xml",
                "application/xml",
                "https://cem.dev/ns/data/xml/1",
                "<message>Hello from CEM</message>\n",
            ));
            config.resources.push(resource(
                "template",
                Role::Template,
                "templates/page.cemt",
                "application/vnd.cem.template+cem",
                "https://cem.dev/ns/template/cem-native/1",
                PIPELINE_TEMPLATE,
            ));
            config.resources.push(resource(
                "pipeline",
                Role::Graph,
                "pipelines/build.cem",
                "application/cem",
                TRANSFORM_CONFIG_SCHEMA_URI,
                PIPELINE_GRAPH,
            ));
            (
                StudioProjectEntryKind::TransformationGraph,
                "Build pipeline",
                vec!["transform", "--config", ".resources/pipeline"],
                Some("generated/result.html"),
            )
        }
        ProjectBootstrapPreset::SimpleConversion => {
            config.resources.push(resource(
                "source",
                Role::Data,
                "data/source.yaml",
                "application/yaml",
                "https://cem.dev/ns/data/yaml/1",
                "message: Hello from CEM\n",
            ));
            (
                StudioProjectEntryKind::Conversion,
                "Convert YAML to CEM-ML",
                vec![
                    "convert",
                    ".resources/source",
                    "--to-content-type",
                    "application/cem",
                    "--to-schema",
                    "https://cem.dev/ns/cem-ml/1",
                    "--tabular",
                    "--output-color-type",
                    "none",
                    "--out",
                    "generated/result.cem",
                ],
                Some("generated/result.cem"),
            )
        }
        ProjectBootstrapPreset::SchemaValidation => {
            config.resources.push(resource("source", Role::Data, "data/document.xhtml", "application/xhtml+xml", crate::schema::registry::XHTML_SCHEMA_URI, "<html xmlns=\"http://www.w3.org/1999/xhtml\" lang=\"en\"><head><title>Validation starter</title></head><body><main><h1>Welcome</h1><p>Validate this document.</p></main></body></html>\n"));
            (
                StudioProjectEntryKind::Validation,
                "Validate document",
                vec!["validate", ".resources/source", "--fail-level", "strict"],
                None,
            )
        }
        ProjectBootstrapPreset::Custom => {
            generate_project(config.clone())?;
            return Ok(config);
        }
    };
    let command = CliCommandResource {
        schema: CLI_COMMAND_SCHEMA_URI.into(),
        schema_version: CLI_COMMAND_SCHEMA_VERSION,
        command_schema_version: CLI_COMMAND_GRAMMAR_SCHEMA_VERSION,
        common_version: crate::VERSION.into(),
        binary_name: CLI_COMMAND_BINARY_NAME.into(),
        argv: argv.into_iter().map(str::to_owned).collect(),
    };
    config.resources.push(resource(
        "command",
        Role::RunConfig,
        "commands/build.json",
        CLI_COMMAND_CONTENT_TYPE,
        CLI_COMMAND_SCHEMA_URI,
        &command
            .serialize(CliCommandProjection::Json)
            .map_err(|e| error("command_invalid", e.to_string(), None))?,
    ));
    config.entries.push(StudioProjectEntry {
        id: "build".into(),
        parent_id: None,
        kind,
        name: title.into(),
        description: None,
        run_config_resource_id: Some("command".into()),
        entry_artifact: artifact.map(str::to_owned),
        resource_ids: config.resources.iter().map(|r| r.id.clone()).collect(),
        tags: vec![],
    });
    generate_project(config.clone())?;
    Ok(config)
}

fn resource(
    id: &str,
    role: StudioProjectResourceRole,
    path: &str,
    content_type: &str,
    schema: &str,
    text: &str,
) -> ProjectBootstrapResource {
    ProjectBootstrapResource {
        id: id.into(),
        role,
        path: path.into(),
        content_type: content_type.into(),
        schema: Some(schema.into()),
        text: text.into(),
    }
}

const PIPELINE_GRAPH: &str = "@doc cem-ml 1\n{run | {import @id=source @src=source @content-type=application/xml @schema=https://cem.dev/ns/data/xml/1 | {transform @id=view @src=template @entrypoint=main @template-content-type=application/vnd.cem.template+cem @template-schema=https://cem.dev/ns/template/cem-native/1 | {export @id=page @out=../generated/result.html @content-type=text/html @schema=https://cem.dev/ns/data/html/1}}}}\n";
const PIPELINE_TEMPLATE: &str = "@doc cem-ml 1\n@ns template = \"https://cem.dev/ns/template/cem-native/1\"\n@ns cem = \"https://cem.dev/ns/core/1\"\n@default template\n{param @name=input @required=true}\n{template @name=main @visibility=public @cem:whitespace=layout | {element @name=main @namespace=http://www.w3.org/1999/xhtml | {element @name=h1 @namespace=http://www.w3.org/1999/xhtml | ```CEM pipeline```} {element @name=p @namespace=http://www.w3.org/1999/xhtml | {$ | dom:text(input)}}}}\n";

/// Reuse the published project entry definitions so host forms cannot drift
/// from native topology. This JSON Schema describes control data, not an AST.
pub fn configuration_schema() -> Value {
    let project: Value = serde_json::from_str(include_str!(
        "../schema-packages/studio-project/v1/schema/studio-project.schema.json"
    ))
    .expect("embedded project schema");
    let mut schema = json!({
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "$id":"https://cem.dev/schema/project-bootstrap/1/configuration.schema.json",
        "title":"CEM project bootstrap v1", "type":"object", "additionalProperties":false,
        "required":["$schema","schemaVersion","commonVersion","preset","projectId","name","createdAt","entries","resources"],
        "properties": {
            "$schema":{"const":PROJECT_BOOTSTRAP_SCHEMA_URI}, "schemaVersion":{"const":PROJECT_BOOTSTRAP_SCHEMA_VERSION},
            "commonVersion":{"const":crate::VERSION},
            "preset":{"enum":["markdown-site","transformation-pipeline","simple-conversion","schema-validation","custom"]},
            "projectId":{"$ref":"#/$defs/id"}, "name":{"$ref":"#/$defs/nonEmptyString"},
            "description":{"$ref":"#/$defs/nonEmptyString"}, "createdAt":{"$ref":"#/$defs/timestamp"},
            "entries":{"type":"array","maxItems":MAX_ENTRIES,"items":{"$ref":"#/$defs/entry"}},
            "resources":{"type":"array","maxItems":MAX_RESOURCES,"items":{
                "type":"object","additionalProperties":false,
                "required":["id","role","path","contentType","text"],
                "properties":{
                    "id":{"$ref":"#/$defs/id"}, "text":{"type":"string"},
                    "role":project["$defs"]["resource"]["properties"]["role"],
                    "path":project["$defs"]["resource"]["properties"]["path"],
                    "contentType":project["$defs"]["resource"]["properties"]["contentType"],
                    "schema":project["$defs"]["resource"]["properties"]["schema"]
                }
            }}
        }
    });
    schema["$defs"] = project["$defs"].clone();
    schema
}
