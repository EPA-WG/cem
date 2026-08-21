//! Portable CEM Studio project v1 model and lossless CEM/JSON projections.
//!
//! Browser persistence, provider bindings, and transient UI state are not part
//! of this module. The same normalized model is used by `project.cem`, its JSON
//! projection, import validation, and later IndexedDB records.

use crate::diagnostics::Severity;
use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::{SchemaTokenKind, SchemaTokenizer};
use crate::transform_template::transform_template_encode_cem_attribute_value;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const STUDIO_PROJECT_CEM_CONTENT_TYPE: &str = "application/vnd.cem.studio-project+cem";
pub const STUDIO_PROJECT_JSON_CONTENT_TYPE: &str = "application/vnd.cem.studio-project+json";
pub const STUDIO_PROJECT_SCHEMA_URI: &str = "https://cem.dev/ns/studio/project/1";
pub const STUDIO_PROJECT_JSON_SCHEMA_URI: &str =
    "https://cem.dev/schema/studio/project.schema.json";
pub const STUDIO_PROJECT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioProjectProjection {
    Cem,
    Json,
}

impl StudioProjectProjection {
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Cem => STUDIO_PROJECT_CEM_CONTENT_TYPE,
            Self::Json => STUDIO_PROJECT_JSON_CONTENT_TYPE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudioProject {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub root_uri: String,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
    pub entries: Vec<StudioProjectEntry>,
    pub resources: Vec<StudioProjectResource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudioProjectEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub kind: StudioProjectEntryKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_config_resource_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StudioProjectEntryKind {
    Subproject,
    DataSet,
    Validation,
    Inspection,
    Conversion,
    Query,
    Transformation,
    TransformationGraph,
    Trace,
}

impl StudioProjectEntryKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "subproject" => Some(Self::Subproject),
            "data-set" => Some(Self::DataSet),
            "validation" => Some(Self::Validation),
            "inspection" => Some(Self::Inspection),
            "conversion" => Some(Self::Conversion),
            "query" => Some(Self::Query),
            "transformation" => Some(Self::Transformation),
            "transformation-graph" => Some(Self::TransformationGraph),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subproject => "subproject",
            Self::DataSet => "data-set",
            Self::Validation => "validation",
            Self::Inspection => "inspection",
            Self::Conversion => "conversion",
            Self::Query => "query",
            Self::Transformation => "transformation",
            Self::TransformationGraph => "transformation-graph",
            Self::Trace => "trace",
        }
    }

    fn requires_run_config(self) -> bool {
        !matches!(self, Self::Subproject | Self::DataSet)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudioProjectResource {
    pub id: String,
    pub role: StudioProjectResourceRole,
    pub source_kind: StudioProjectResourceSourceKind,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub content_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub revision: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StudioProjectResourceRole {
    Data,
    Schema,
    Query,
    Template,
    TransformConfig,
    Graph,
    RunConfig,
    Expected,
}

impl StudioProjectResourceRole {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "data" => Some(Self::Data),
            "schema" => Some(Self::Schema),
            "query" => Some(Self::Query),
            "template" => Some(Self::Template),
            "transform-config" => Some(Self::TransformConfig),
            "graph" => Some(Self::Graph),
            "run-config" => Some(Self::RunConfig),
            "expected" => Some(Self::Expected),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Schema => "schema",
            Self::Query => "query",
            Self::Template => "template",
            Self::TransformConfig => "transform-config",
            Self::Graph => "graph",
            Self::RunConfig => "run-config",
            Self::Expected => "expected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StudioProjectResourceSourceKind {
    ProjectFile,
    Url,
}

impl StudioProjectResourceSourceKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "project-file" => Some(Self::ProjectFile),
            "url" => Some(Self::Url),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectFile => "project-file",
            Self::Url => "url",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioProjectError {
    pub code: &'static str,
    pub message: String,
    pub field_path: Option<String>,
}

impl std::fmt::Display for StudioProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field_path) = &self.field_path {
            write!(formatter, "{} at {field_path}: {}", self.code, self.message)
        } else {
            write!(formatter, "{}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for StudioProjectError {}

impl StudioProject {
    pub fn serialize(
        &self,
        projection: StudioProjectProjection,
    ) -> Result<String, StudioProjectError> {
        validate_studio_project(self)?;
        match projection {
            StudioProjectProjection::Cem => serialize_studio_project_cem(self),
            StudioProjectProjection::Json => {
                let mut output = serde_json::to_string_pretty(self).map_err(|error| {
                    studio_error(
                        "cem.studio_project.json_serialization_failed",
                        format!("Studio project JSON serialization failed: {error}"),
                        None,
                    )
                })?;
                output.push('\n');
                Ok(output)
            }
        }
    }
}

pub fn parse_studio_project(
    bytes: &[u8],
    content_type: &str,
    schema: &str,
) -> Result<StudioProject, StudioProjectError> {
    if schema.trim() != STUDIO_PROJECT_SCHEMA_URI {
        return Err(studio_error(
            "cem.studio_project.schema_identity_unsupported",
            format!(
                "Studio project schema `{}` is unsupported; expected `{STUDIO_PROJECT_SCHEMA_URI}`",
                schema.trim()
            ),
            Some("$schema"),
        ));
    }

    let content_type = content_type_essence(content_type);
    let project = match content_type.as_str() {
        STUDIO_PROJECT_CEM_CONTENT_TYPE => parse_studio_project_cem(bytes)?,
        STUDIO_PROJECT_JSON_CONTENT_TYPE => {
            serde_json::from_slice::<StudioProject>(bytes).map_err(|error| {
                studio_error(
                    "cem.studio_project.invalid_json",
                    format!("Studio project JSON could not be parsed: {error}"),
                    None,
                )
            })?
        }
        other => {
            return Err(studio_error(
                "cem.studio_project.content_type_unsupported",
                format!(
                    "Studio project content type `{other}` is unsupported; expected `{STUDIO_PROJECT_CEM_CONTENT_TYPE}` or `{STUDIO_PROJECT_JSON_CONTENT_TYPE}`"
                ),
                None,
            ))
        }
    };
    validate_studio_project(&project)?;
    Ok(project)
}

pub fn studio_project_resource_uri(
    project: &StudioProject,
    resource_id: &str,
) -> Result<String, StudioProjectError> {
    validate_studio_project(project)?;
    let resource = project
        .resources
        .iter()
        .find(|resource| resource.id == resource_id)
        .ok_or_else(|| {
            studio_error(
                "cem.studio_project.resource_reference_unresolved",
                format!("resource `{resource_id}` is not declared"),
                Some("resourceId"),
            )
        })?;
    Ok(format!("{}{}", project.root_uri, resource.path))
}

pub fn validate_studio_project(project: &StudioProject) -> Result<(), StudioProjectError> {
    if project.schema != STUDIO_PROJECT_SCHEMA_URI {
        return Err(studio_error(
            "cem.studio_project.schema_identity_unsupported",
            format!(
                "Studio project JSON $schema `{}` is unsupported; expected `{STUDIO_PROJECT_SCHEMA_URI}`",
                project.schema
            ),
            Some("$schema"),
        ));
    }
    if project.schema_version != STUDIO_PROJECT_SCHEMA_VERSION {
        return Err(studio_error(
            "cem.studio_project.schema_version_unsupported",
            format!(
                "Studio project schemaVersion {} is unsupported; expected {}",
                project.schema_version, STUDIO_PROJECT_SCHEMA_VERSION
            ),
            Some("schemaVersion"),
        ));
    }
    validate_identifier(&project.id, "id")?;
    validate_text(&project.name, "name")?;
    validate_optional_text(project.description.as_deref(), "description")?;
    let expected_root_uri = format!("studio://{}/", project.id);
    if project.root_uri != expected_root_uri {
        return Err(studio_error(
            "cem.studio_project.root_uri_invalid",
            format!(
                "rootUri `{}` does not match the project id; expected `{expected_root_uri}`",
                project.root_uri
            ),
            Some("rootUri"),
        ));
    }
    validate_revision(project.revision, "revision")?;
    validate_timestamp(&project.created_at, "createdAt")?;
    validate_timestamp(&project.updated_at, "updatedAt")?;

    let mut ids = BTreeSet::from([project.id.as_str()]);
    let entries = project
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let resources = project
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource))
        .collect::<BTreeMap<_, _>>();

    for (index, entry) in project.entries.iter().enumerate() {
        let path = format!("entries[{index}]");
        validate_identifier(&entry.id, &format!("{path}.id"))?;
        if !ids.insert(entry.id.as_str()) {
            return Err(studio_error(
                "cem.studio_project.id_duplicate",
                format!("stable id `{}` is declared more than once", entry.id),
                Some(&format!("{path}.id")),
            ));
        }
        validate_text(&entry.name, &format!("{path}.name"))?;
        validate_optional_text(entry.description.as_deref(), &format!("{path}.description"))?;
        validate_unique_identifiers(&entry.resource_ids, &format!("{path}.resourceIds"))?;
        validate_unique_text(&entry.tags, &format!("{path}.tags"))?;
        if entry.kind.requires_run_config() && entry.run_config_resource_id.is_none() {
            return Err(studio_error(
                "cem.studio_project.run_config_required",
                format!(
                    "{} entry `{}` requires a run configuration",
                    entry.kind.as_str(),
                    entry.id
                ),
                Some(&format!("{path}.runConfigResourceId")),
            ));
        }
        if !entry.kind.requires_run_config() && entry.run_config_resource_id.is_some() {
            return Err(studio_error(
                "cem.studio_project.run_config_forbidden",
                format!(
                    "{} entry `{}` cannot own a run configuration",
                    entry.kind.as_str(),
                    entry.id
                ),
                Some(&format!("{path}.runConfigResourceId")),
            ));
        }
        if entry.kind == StudioProjectEntryKind::DataSet && entry.resource_ids.is_empty() {
            return Err(studio_error(
                "cem.studio_project.data_set_empty",
                format!(
                    "data-set entry `{}` must reference at least one resource",
                    entry.id
                ),
                Some(&format!("{path}.resourceIds")),
            ));
        }
    }

    for (index, resource) in project.resources.iter().enumerate() {
        let path = format!("resources[{index}]");
        validate_identifier(&resource.id, &format!("{path}.id"))?;
        if !ids.insert(resource.id.as_str()) {
            return Err(studio_error(
                "cem.studio_project.id_duplicate",
                format!("stable id `{}` is declared more than once", resource.id),
                Some(&format!("{path}.id")),
            ));
        }
        validate_resource_path(&resource.path, &format!("{path}.path"))?;
        validate_content_type(&resource.content_type, &format!("{path}.contentType"))?;
        validate_optional_uri(resource.schema.as_deref(), &format!("{path}.schema"))?;
        validate_revision(resource.revision, &format!("{path}.revision"))?;
        validate_sha256(&resource.sha256, &format!("{path}.sha256"))?;
        match resource.source_kind {
            StudioProjectResourceSourceKind::ProjectFile if resource.url.is_some() => {
                return Err(studio_error(
                    "cem.studio_project.resource_source_invalid",
                    "project-file resources cannot declare a URL",
                    Some(&format!("{path}.url")),
                ));
            }
            StudioProjectResourceSourceKind::ProjectFile => {}
            StudioProjectResourceSourceKind::Url => {
                let url = resource.url.as_deref().ok_or_else(|| {
                    studio_error(
                        "cem.studio_project.resource_source_invalid",
                        "URL resources must declare url",
                        Some(&format!("{path}.url")),
                    )
                })?;
                validate_remote_url(url, &format!("{path}.url"))?;
            }
        }
    }

    for (index, entry) in project.entries.iter().enumerate() {
        let path = format!("entries[{index}]");
        if let Some(parent_id) = entry.parent_id.as_deref() {
            let parent = entries.get(parent_id).ok_or_else(|| {
                studio_error(
                    "cem.studio_project.parent_reference_unresolved",
                    format!(
                        "entry `{}` references missing parent `{parent_id}`",
                        entry.id
                    ),
                    Some(&format!("{path}.parentId")),
                )
            })?;
            if parent.kind != StudioProjectEntryKind::Subproject {
                return Err(studio_error(
                    "cem.studio_project.parent_kind_invalid",
                    format!(
                        "entry `{}` parent `{parent_id}` is not a subproject",
                        entry.id
                    ),
                    Some(&format!("{path}.parentId")),
                ));
            }
        }
        validate_parent_chain(entry, &entries, &path)?;
        for resource_id in &entry.resource_ids {
            if !resources.contains_key(resource_id.as_str()) {
                return Err(studio_error(
                    "cem.studio_project.resource_reference_unresolved",
                    format!(
                        "entry `{}` references missing resource `{resource_id}`",
                        entry.id
                    ),
                    Some(&format!("{path}.resourceIds")),
                ));
            }
        }
        if let Some(run_config_id) = entry.run_config_resource_id.as_deref() {
            let resource = resources.get(run_config_id).ok_or_else(|| {
                studio_error(
                    "cem.studio_project.resource_reference_unresolved",
                    format!(
                        "entry `{}` references missing run config `{run_config_id}`",
                        entry.id
                    ),
                    Some(&format!("{path}.runConfigResourceId")),
                )
            })?;
            if resource.role != StudioProjectResourceRole::RunConfig {
                return Err(studio_error(
                    "cem.studio_project.run_config_role_invalid",
                    format!("resource `{run_config_id}` is not a run-config resource"),
                    Some(&format!("{path}.runConfigResourceId")),
                ));
            }
        }
    }

    Ok(())
}

fn parse_studio_project_cem(bytes: &[u8]) -> Result<StudioProject, StudioProjectError> {
    let input = std::str::from_utf8(bytes).map_err(|error| {
        studio_error(
            "cem.studio_project.invalid_cem",
            format!("Studio project CEM is not valid UTF-8: {error}"),
            None,
        )
    })?;
    validate_studio_project_cem_identity(input)?;
    let source = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tokenizer = CemTokenizer::from_source(source);
    let normalizer = CemEventNormalizer::new(tokenizer);
    let document = CemAstBuilder::new(normalizer).top_level(true).build();
    if let Some(diagnostic) = document
        .diagnostics
        .iter()
        .find(|diagnostic| matches!(diagnostic.severity, Severity::Error | Severity::Fatal))
    {
        return Err(studio_error(
            "cem.studio_project.invalid_cem",
            format!("{}: {}", diagnostic.code, diagnostic.message),
            None,
        ));
    }

    let project_id = element_ids(&document, None, "project")
        .into_iter()
        .next()
        .ok_or_else(|| {
            studio_error(
                "cem.studio_project.project_missing",
                "Studio project CEM must contain one project root",
                None,
            )
        })?;
    if element_ids(&document, None, "project").len() != 1 {
        return Err(studio_error(
            "cem.studio_project.project_duplicate",
            "Studio project CEM must contain exactly one project root",
            None,
        ));
    }

    let project_attrs = strict_attrs(
        &document,
        project_id,
        &[
            "schema-version",
            "id",
            "name",
            "description",
            "root-uri",
            "revision",
            "created-at",
            "updated-at",
        ],
        "project",
    )?;
    let mut entries = Vec::new();
    let mut resources = Vec::new();
    let Some(CemAstNode::Element { children, .. }) = document.get(project_id) else {
        unreachable!("project id resolves to an element")
    };
    for child_id in children {
        let Some(CemAstNode::Element { expanded_name, .. }) = document.get(*child_id) else {
            continue;
        };
        match expanded_name.local_name.as_str() {
            "entry" => entries.push(parse_cem_entry(&document, *child_id, entries.len())?),
            "resource" => {
                resources.push(parse_cem_resource(&document, *child_id, resources.len())?)
            }
            other => {
                return Err(studio_error(
                    "cem.studio_project.child_unknown",
                    format!("Studio project child `{other}` is not allowed"),
                    Some("project"),
                ))
            }
        }
    }

    Ok(StudioProject {
        schema: STUDIO_PROJECT_SCHEMA_URI.to_owned(),
        schema_version: required_u32(&project_attrs, "schema-version", "schemaVersion")?,
        id: required_attr(&project_attrs, "id", "id")?,
        name: required_attr(&project_attrs, "name", "name")?,
        description: project_attrs.get("description").cloned(),
        root_uri: required_attr(&project_attrs, "root-uri", "rootUri")?,
        revision: required_u64(&project_attrs, "revision", "revision")?,
        created_at: required_attr(&project_attrs, "created-at", "createdAt")?,
        updated_at: required_attr(&project_attrs, "updated-at", "updatedAt")?,
        entries,
        resources,
    })
}

fn validate_studio_project_cem_identity(input: &str) -> Result<(), StudioProjectError> {
    let source = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let mut tokenizer = CemTokenizer::from_source(source);
    let mut schema_prefixes = BTreeSet::new();
    let mut default_prefixes = Vec::new();

    while let Some(token) = tokenizer.next_token() {
        let SchemaTokenKind::Directive { name, data } = token.kind else {
            continue;
        };
        match name.as_str() {
            "ns" => {
                if let Some((prefix, namespace_uri)) = parse_cem_namespace_binding(&data) {
                    if namespace_uri == STUDIO_PROJECT_SCHEMA_URI {
                        schema_prefixes.insert(prefix.to_owned());
                    }
                }
            }
            "default" => default_prefixes.push(data.trim().to_owned()),
            _ => {}
        }
    }

    if default_prefixes.len() == 1 && schema_prefixes.contains(&default_prefixes[0]) {
        Ok(())
    } else {
        Err(studio_error(
            "cem.studio_project.schema_identity_unsupported",
            format!(
                "Studio project CEM must bind `{STUDIO_PROJECT_SCHEMA_URI}` with @ns and select that prefix with exactly one @default directive"
            ),
            Some("@ns"),
        ))
    }
}

fn parse_cem_namespace_binding(data: &str) -> Option<(&str, &str)> {
    let (prefix, value) = data.split_once('=')?;
    let prefix = prefix.trim();
    let value = value.trim();
    if prefix.is_empty() {
        return None;
    }
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .map(|namespace_uri| (prefix, namespace_uri))
}

fn parse_cem_entry(
    document: &CemDocument,
    node_id: AstNodeId,
    index: usize,
) -> Result<StudioProjectEntry, StudioProjectError> {
    let path = format!("entries[{index}]");
    let attrs = strict_attrs(
        document,
        node_id,
        &[
            "id",
            "parent-id",
            "kind",
            "name",
            "description",
            "run-config-resource-id",
            "resource-ids",
            "tags",
        ],
        &path,
    )?;
    let kind = required_attr(&attrs, "kind", &format!("{path}.kind"))?;
    let kind = StudioProjectEntryKind::parse(&kind).ok_or_else(|| {
        studio_error(
            "cem.studio_project.entry_kind_invalid",
            format!("entry kind `{kind}` is unsupported"),
            Some(&format!("{path}.kind")),
        )
    })?;
    Ok(StudioProjectEntry {
        id: required_attr(&attrs, "id", &format!("{path}.id"))?,
        parent_id: attrs.get("parent-id").cloned(),
        kind,
        name: required_attr(&attrs, "name", &format!("{path}.name"))?,
        description: attrs.get("description").cloned(),
        run_config_resource_id: attrs.get("run-config-resource-id").cloned(),
        resource_ids: split_list(attrs.get("resource-ids")),
        tags: split_list(attrs.get("tags")),
    })
}

fn parse_cem_resource(
    document: &CemDocument,
    node_id: AstNodeId,
    index: usize,
) -> Result<StudioProjectResource, StudioProjectError> {
    let path = format!("resources[{index}]");
    let attrs = strict_attrs(
        document,
        node_id,
        &[
            "id",
            "role",
            "source-kind",
            "path",
            "url",
            "content-type",
            "schema",
            "revision",
            "sha256",
        ],
        &path,
    )?;
    let role = required_attr(&attrs, "role", &format!("{path}.role"))?;
    let role = StudioProjectResourceRole::parse(&role).ok_or_else(|| {
        studio_error(
            "cem.studio_project.resource_role_invalid",
            format!("resource role `{role}` is unsupported"),
            Some(&format!("{path}.role")),
        )
    })?;
    let source_kind = required_attr(&attrs, "source-kind", &format!("{path}.sourceKind"))?;
    let source_kind = StudioProjectResourceSourceKind::parse(&source_kind).ok_or_else(|| {
        studio_error(
            "cem.studio_project.resource_source_invalid",
            format!("resource source kind `{source_kind}` is unsupported"),
            Some(&format!("{path}.sourceKind")),
        )
    })?;
    Ok(StudioProjectResource {
        id: required_attr(&attrs, "id", &format!("{path}.id"))?,
        role,
        source_kind,
        path: required_attr(&attrs, "path", &format!("{path}.path"))?,
        url: attrs.get("url").cloned(),
        content_type: required_attr(&attrs, "content-type", &format!("{path}.contentType"))?,
        schema: attrs.get("schema").cloned(),
        revision: required_u64(&attrs, "revision", &format!("{path}.revision"))?,
        sha256: required_attr(&attrs, "sha256", &format!("{path}.sha256"))?,
    })
}

fn serialize_studio_project_cem(project: &StudioProject) -> Result<String, StudioProjectError> {
    let mut output = format!(
        "@doc cem-ml 1\n@ns studio = \"{STUDIO_PROJECT_SCHEMA_URI}\"\n@default studio\n\n{{project"
    );
    push_cem_attr(
        &mut output,
        "schema-version",
        &project.schema_version.to_string(),
    )?;
    push_cem_attr(&mut output, "id", &project.id)?;
    push_cem_attr(&mut output, "name", &project.name)?;
    if let Some(description) = &project.description {
        push_cem_attr(&mut output, "description", description)?;
    }
    push_cem_attr(&mut output, "root-uri", &project.root_uri)?;
    push_cem_attr(&mut output, "revision", &project.revision.to_string())?;
    push_cem_attr(&mut output, "created-at", &project.created_at)?;
    push_cem_attr(&mut output, "updated-at", &project.updated_at)?;
    output.push_str(" |\n");
    for entry in &project.entries {
        output.push_str("    {entry");
        push_cem_attr(&mut output, "id", &entry.id)?;
        if let Some(parent_id) = &entry.parent_id {
            push_cem_attr(&mut output, "parent-id", parent_id)?;
        }
        push_cem_attr(&mut output, "kind", entry.kind.as_str())?;
        push_cem_attr(&mut output, "name", &entry.name)?;
        if let Some(description) = &entry.description {
            push_cem_attr(&mut output, "description", description)?;
        }
        if let Some(run_config_resource_id) = &entry.run_config_resource_id {
            push_cem_attr(
                &mut output,
                "run-config-resource-id",
                run_config_resource_id,
            )?;
        }
        if !entry.resource_ids.is_empty() {
            push_cem_attr(&mut output, "resource-ids", &entry.resource_ids.join(" "))?;
        }
        if !entry.tags.is_empty() {
            push_cem_attr(&mut output, "tags", &entry.tags.join(" "))?;
        }
        output.push_str("}\n");
    }
    for resource in &project.resources {
        output.push_str("    {resource");
        push_cem_attr(&mut output, "id", &resource.id)?;
        push_cem_attr(&mut output, "role", resource.role.as_str())?;
        push_cem_attr(&mut output, "source-kind", resource.source_kind.as_str())?;
        push_cem_attr(&mut output, "path", &resource.path)?;
        if let Some(url) = &resource.url {
            push_cem_attr(&mut output, "url", url)?;
        }
        push_cem_attr(&mut output, "content-type", &resource.content_type)?;
        if let Some(schema) = &resource.schema {
            push_cem_attr(&mut output, "schema", schema)?;
        }
        push_cem_attr(&mut output, "revision", &resource.revision.to_string())?;
        push_cem_attr(&mut output, "sha256", &resource.sha256)?;
        output.push_str("}\n");
    }
    output.push_str("}\n");
    Ok(output)
}

fn push_cem_attr(output: &mut String, name: &str, value: &str) -> Result<(), StudioProjectError> {
    let encoded = transform_template_encode_cem_attribute_value(value, "Studio project attribute")
        .map_err(|message| {
            studio_error(
                "cem.studio_project.cem_serialization_failed",
                message,
                Some(name),
            )
        })?;
    output.push_str(" @");
    output.push_str(name);
    output.push('=');
    output.push_str(&encoded);
    Ok(())
}

fn strict_attrs(
    document: &CemDocument,
    node_id: AstNodeId,
    allowed: &[&str],
    field_path: &str,
) -> Result<BTreeMap<String, String>, StudioProjectError> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = document.get(node_id) else {
        return Ok(attrs);
    };
    for attribute_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attribute_id)
        else {
            continue;
        };
        if !allowed.contains(&expanded_name.local_name.as_str()) {
            return Err(studio_error(
                "cem.studio_project.attribute_unknown",
                format!("attribute `{}` is not allowed", expanded_name.local_name),
                Some(field_path),
            ));
        }
        if attrs
            .insert(
                expanded_name.local_name.clone(),
                value.clone().unwrap_or_default(),
            )
            .is_some()
        {
            return Err(studio_error(
                "cem.studio_project.attribute_duplicate",
                format!(
                    "attribute `{}` is declared more than once",
                    expanded_name.local_name
                ),
                Some(field_path),
            ));
        }
    }
    Ok(attrs)
}

fn element_ids(
    document: &CemDocument,
    parent: Option<AstNodeId>,
    local_name: &str,
) -> Vec<AstNodeId> {
    let candidates: Vec<AstNodeId> = match parent.and_then(|id| document.get(id)) {
        Some(CemAstNode::Element { children, .. }) => children.clone(),
        _ => document
            .iter()
            .filter_map(|node| match node {
                CemAstNode::Element { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .collect(),
    };
    candidates
        .into_iter()
        .filter(|node_id| {
            matches!(
                document.get(*node_id),
                Some(CemAstNode::Element { expanded_name, .. })
                    if expanded_name.local_name == local_name
            )
        })
        .collect()
}

fn required_attr(
    attrs: &BTreeMap<String, String>,
    name: &str,
    field_path: &str,
) -> Result<String, StudioProjectError> {
    attrs
        .get(name)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| {
            studio_error(
                "cem.studio_project.field_required",
                format!("required field `{name}` is missing"),
                Some(field_path),
            )
        })
}

fn required_u32(
    attrs: &BTreeMap<String, String>,
    name: &str,
    field_path: &str,
) -> Result<u32, StudioProjectError> {
    required_attr(attrs, name, field_path)?
        .parse::<u32>()
        .map_err(|_| {
            studio_error(
                "cem.studio_project.field_invalid",
                format!("field `{name}` must be an unsigned integer"),
                Some(field_path),
            )
        })
}

fn required_u64(
    attrs: &BTreeMap<String, String>,
    name: &str,
    field_path: &str,
) -> Result<u64, StudioProjectError> {
    required_attr(attrs, name, field_path)?
        .parse::<u64>()
        .map_err(|_| {
            studio_error(
                "cem.studio_project.field_invalid",
                format!("field `{name}` must be an unsigned integer"),
                Some(field_path),
            )
        })
}

fn split_list(value: Option<&String>) -> Vec<String> {
    value
        .map(|value| value.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn validate_identifier(value: &str, field_path: &str) -> Result<(), StudioProjectError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(studio_error(
            "cem.studio_project.id_invalid",
            format!("`{value}` is not a lowercase stable id"),
            Some(field_path),
        ))
    }
}

fn validate_text(value: &str, field_path: &str) -> Result<(), StudioProjectError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        Err(studio_error(
            "cem.studio_project.text_invalid",
            "text must be non-empty and cannot contain control characters",
            Some(field_path),
        ))
    } else {
        Ok(())
    }
}

fn validate_optional_text(value: Option<&str>, field_path: &str) -> Result<(), StudioProjectError> {
    match value {
        Some(value) => validate_text(value, field_path),
        None => Ok(()),
    }
}

fn validate_revision(value: u64, field_path: &str) -> Result<(), StudioProjectError> {
    if value == 0 {
        Err(studio_error(
            "cem.studio_project.revision_invalid",
            "revision must be greater than zero",
            Some(field_path),
        ))
    } else {
        Ok(())
    }
}

fn validate_timestamp(value: &str, field_path: &str) -> Result<(), StudioProjectError> {
    let valid = value.len() >= 20 && value.contains('T') && value.ends_with('Z');
    if valid {
        Ok(())
    } else {
        Err(studio_error(
            "cem.studio_project.timestamp_invalid",
            format!("timestamp `{value}` must be an RFC 3339 UTC value"),
            Some(field_path),
        ))
    }
}

fn validate_resource_path(value: &str, field_path: &str) -> Result<(), StudioProjectError> {
    let valid = !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains('?')
        && !value.contains('#')
        && value.split('/').all(|segment| {
            !segment.is_empty() && segment != "." && segment != ".." && segment != ".cem-studio"
        });
    if valid {
        Ok(())
    } else {
        Err(studio_error(
            "cem.studio_project.resource_path_invalid",
            format!("resource path `{value}` must be a contained relative project path"),
            Some(field_path),
        ))
    }
}

fn validate_content_type(value: &str, field_path: &str) -> Result<(), StudioProjectError> {
    let essence = content_type_essence(value);
    let valid = essence
        .split_once('/')
        .is_some_and(|(kind, subtype)| !kind.is_empty() && !subtype.is_empty())
        && !essence.chars().any(char::is_whitespace);
    if valid {
        Ok(())
    } else {
        Err(studio_error(
            "cem.studio_project.content_type_invalid",
            format!("resource content type `{value}` is invalid"),
            Some(field_path),
        ))
    }
}

fn validate_optional_uri(value: Option<&str>, field_path: &str) -> Result<(), StudioProjectError> {
    if value.is_some_and(|value| !value.contains(':') || value.chars().any(char::is_whitespace)) {
        Err(studio_error(
            "cem.studio_project.schema_identity_invalid",
            "schema identity must be an absolute URI without whitespace",
            Some(field_path),
        ))
    } else {
        Ok(())
    }
}

fn validate_sha256(value: &str, field_path: &str) -> Result<(), StudioProjectError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(studio_error(
            "cem.studio_project.sha256_invalid",
            "sha256 must contain exactly 64 lowercase hexadecimal characters",
            Some(field_path),
        ))
    }
}

fn validate_remote_url(value: &str, field_path: &str) -> Result<(), StudioProjectError> {
    let Some((scheme, remainder)) = value.split_once("://") else {
        return Err(studio_error(
            "cem.studio_project.resource_url_invalid",
            "URL resource must use an explicit http or https URL",
            Some(field_path),
        ));
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    if !matches!(scheme, "http" | "https")
        || authority.is_empty()
        || authority.contains('@')
        || value.chars().any(char::is_whitespace)
    {
        return Err(studio_error(
            "cem.studio_project.resource_url_invalid",
            "URL resource must use http/https without embedded credentials or whitespace",
            Some(field_path),
        ));
    }
    Ok(())
}

fn validate_unique_identifiers(
    values: &[String],
    field_path: &str,
) -> Result<(), StudioProjectError> {
    for value in values {
        validate_identifier(value, field_path)?;
    }
    validate_unique_text(values, field_path)
}

fn validate_unique_text(values: &[String], field_path: &str) -> Result<(), StudioProjectError> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_text(value, field_path)?;
        if !seen.insert(value.as_str()) {
            return Err(studio_error(
                "cem.studio_project.list_value_duplicate",
                format!("list value `{value}` is duplicated"),
                Some(field_path),
            ));
        }
    }
    Ok(())
}

fn validate_parent_chain(
    entry: &StudioProjectEntry,
    entries: &BTreeMap<&str, &StudioProjectEntry>,
    field_path: &str,
) -> Result<(), StudioProjectError> {
    let mut seen = BTreeSet::from([entry.id.as_str()]);
    let mut parent_id = entry.parent_id.as_deref();
    while let Some(id) = parent_id {
        if !seen.insert(id) {
            return Err(studio_error(
                "cem.studio_project.hierarchy_cycle",
                format!("entry `{}` participates in a parent cycle", entry.id),
                Some(&format!("{field_path}.parentId")),
            ));
        }
        parent_id = entries
            .get(id)
            .and_then(|parent| parent.parent_id.as_deref());
    }
    Ok(())
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn studio_error(
    code: &'static str,
    message: impl Into<String>,
    field_path: Option<&str>,
) -> StudioProjectError {
    StudioProjectError {
        code,
        message: message.into(),
        field_path: field_path.map(str::to_owned),
    }
}
