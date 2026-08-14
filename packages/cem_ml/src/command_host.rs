//! Constructor-owned host capabilities for command-service orchestration.
//!
//! Wire requests contain resource identities and optional inline snapshots, but
//! never callbacks. This layer admits a request against the host's revision
//! ledger, deterministically reads only the missing snapshots needed by native
//! operation preparation, and verifies the returned revision and digest before
//! adding bytes to an owned request. Publication remains a separate capability.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::command_service::{
    admit_command_service_request_v1, sha256_hex, validate_command_service_request_v1,
    CommandProjectRevisionV1, CommandResourceVersionV1, CommandRevisionLedgerV1,
    CommandServiceAdmissionV1, CommandServiceError, CommandServiceLimitsV1,
    CommandServiceRequestV1, CommandStaleRevisionV1, CommandTransformSourceV1,
    PortableOperationRequestV1, VirtualResourceV1,
};
use crate::engine::FormatIdentity;
use crate::operation_control::{ControlError, OperationControl};
use crate::resolver::ResolvePurpose;
use crate::run_config::{infer_content_type_from_path, NormalizedRunPlan};
use crate::schema::registry::XPATH_CONTENT_TYPE;
use crate::transform_config::{
    parse_transform_graph_config, TransformGraphNodeKind, TransformGraphParseRequest,
};

/// A host callback future deliberately carries no `Send` requirement so the
/// same capability can wrap browser promises and native async readers. Native
/// adapters remain free to return a `Send` future through this erased boundary.
pub type CommandHostFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResourceReadRequestV1 {
    pub request_id: String,
    pub project: CommandProjectRevisionV1,
    pub uri: String,
    pub purposes: BTreeSet<ResolvePurpose>,
    pub content_type_hints: BTreeSet<String>,
    pub expected: CommandResourceVersionV1,
    pub resolver_policy_stamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResolvedResourceV1 {
    pub version: CommandResourceVersionV1,
    pub bytes: Vec<u8>,
    pub identity: Option<FormatIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResourceReadFailureV1 {
    pub code: String,
    pub message: String,
}

impl CommandResourceReadFailureV1 {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for CommandResourceReadFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommandResourceReadFailureV1 {}

/// Constructor-time read capability. Implementations may call a browser
/// promise, Node adapter, native resolver, or an in-memory fixture.
pub trait CommandResourceReaderV1 {
    fn read<'a>(
        &'a self,
        request: CommandResourceReadRequestV1,
    ) -> CommandHostFuture<'a, Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>>;
}

#[derive(Debug, Clone)]
pub enum CommandServiceHydrationV1 {
    Ready(Box<CommandServiceRequestV1>),
    Stale(CommandStaleRevisionV1),
}

impl CommandServiceHydrationV1 {
    pub fn ready(self) -> Option<Box<CommandServiceRequestV1>> {
        match self {
            Self::Ready(request) => Some(request),
            Self::Stale(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResourceHydrationErrorV1 {
    Request(CommandServiceError),
    HostRead {
        uri: String,
        source: CommandResourceReadFailureV1,
    },
    ResourceRevisionMismatch {
        uri: String,
        expected: u64,
        actual: u64,
    },
    ResourceDigestMismatch {
        uri: String,
        expected: String,
        actual: String,
    },
    ResourceBytesDigestMismatch {
        uri: String,
        expected: String,
        actual: String,
    },
    GraphResourceManifest {
        uri: String,
        message: String,
    },
    Control(ControlError),
}

impl CommandResourceHydrationErrorV1 {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request(error) => error.code(),
            Self::HostRead { .. } => "cem.command_service.host_read",
            Self::ResourceRevisionMismatch { .. } => {
                "cem.command_service.host_resource_revision_mismatch"
            }
            Self::ResourceDigestMismatch { .. } => {
                "cem.command_service.host_resource_digest_mismatch"
            }
            Self::ResourceBytesDigestMismatch { .. } => {
                "cem.command_service.host_resource_bytes_digest_mismatch"
            }
            Self::GraphResourceManifest { .. } => "cem.command_service.graph_resource_manifest",
            Self::Control(error) => error.code(),
        }
    }
}

impl fmt::Display for CommandResourceHydrationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => error.fmt(formatter),
            Self::HostRead { uri, source } => {
                write!(formatter, "host read for `{uri}` failed: {source}")
            }
            Self::ResourceRevisionMismatch {
                uri,
                expected,
                actual,
            } => write!(
                formatter,
                "host read for `{uri}` returned revision {actual}; expected {expected}"
            ),
            Self::ResourceDigestMismatch {
                uri,
                expected,
                actual,
            } => write!(
                formatter,
                "host read for `{uri}` declared digest `{actual}`; expected `{expected}`"
            ),
            Self::ResourceBytesDigestMismatch {
                uri,
                expected,
                actual,
            } => write!(
                formatter,
                "host read bytes for `{uri}` hash to `{actual}`; expected `{expected}`"
            ),
            Self::GraphResourceManifest { uri, message } => {
                write!(
                    formatter,
                    "transform graph `{uri}` resource manifest is invalid: {message}"
                )
            }
            Self::Control(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CommandResourceHydrationErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::HostRead { source, .. } => Some(source),
            Self::Control(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CommandServiceError> for CommandResourceHydrationErrorV1 {
    fn from(error: CommandServiceError) -> Self {
        Self::Request(error)
    }
}

impl From<ControlError> for CommandResourceHydrationErrorV1 {
    fn from(error: ControlError) -> Self {
        Self::Control(error)
    }
}

#[derive(Debug, Default)]
struct ResourceNeed {
    purposes: BTreeSet<ResolvePurpose>,
    content_type_hints: BTreeSet<String>,
}

/// Return the deterministic, URI-sorted host reads needed before native
/// operation preparation. Existing inline resources bypass the host reader.
pub fn missing_command_resource_reads_v1(
    request: &CommandServiceRequestV1,
    limits: CommandServiceLimitsV1,
) -> Result<Vec<CommandResourceReadRequestV1>, CommandResourceHydrationErrorV1> {
    validate_command_service_request_v1(request, limits)?;
    if matches!(
        request.operation,
        PortableOperationRequestV1::VersionCapabilities
    ) {
        return Ok(Vec::new());
    }

    let plan = request
        .run_plan
        .plan()
        .expect("validated non-capability request has a normalized run plan");
    let mut needs = BTreeMap::<String, ResourceNeed>::new();

    for package in &plan.schema_packages {
        add_need(
            request,
            &mut needs,
            package
                .resolved_uri
                .as_deref()
                .unwrap_or(&package.declared_uri),
            ResolvePurpose::Input,
            package.identity.content_type.as_deref(),
        );
    }

    match &request.operation {
        PortableOperationRequestV1::Parse { input_id, .. }
        | PortableOperationRequestV1::Inspect { input_id, .. }
        | PortableOperationRequestV1::Convert { input_id, .. }
        | PortableOperationRequestV1::Trace { input_id, .. } => {
            add_input_need(request, plan, &mut needs, input_id)?;
        }
        PortableOperationRequestV1::Validate { input_ids, .. }
        | PortableOperationRequestV1::Check { input_ids, .. } => {
            for input_id in input_ids {
                add_input_need(request, plan, &mut needs, input_id)?;
            }
        }
        PortableOperationRequestV1::Query {
            data_input_id,
            query_uri,
            ..
        } => {
            add_input_need(request, plan, &mut needs, data_input_id)?;
            add_need(
                request,
                &mut needs,
                query_uri,
                ResolvePurpose::Query,
                query_content_type_hint(query_uri).as_deref(),
            );
        }
        PortableOperationRequestV1::Transform { source, .. } => match source {
            CommandTransformSourceV1::Direct {
                data_input_id,
                template_uri,
            } => {
                add_input_need(request, plan, &mut needs, data_input_id)?;
                add_need(
                    request,
                    &mut needs,
                    template_uri,
                    ResolvePurpose::Template,
                    infer_content_type_from_path(template_uri).as_deref(),
                );
            }
            CommandTransformSourceV1::Graph { config_uri } => add_need(
                request,
                &mut needs,
                config_uri,
                ResolvePurpose::Config,
                Some("application/cem+xml"),
            ),
        },
        PortableOperationRequestV1::VersionCapabilities => unreachable!(),
    }

    needs
        .into_iter()
        .map(|(uri, need)| {
            let expected = request
                .resource_versions
                .get(&uri)
                .cloned()
                .ok_or_else(|| CommandServiceError::ResourceVersionMissing { uri: uri.clone() })?;
            Ok(CommandResourceReadRequestV1 {
                request_id: request.request_id.clone(),
                project: request.project.clone(),
                uri,
                purposes: need.purposes,
                content_type_hints: need.content_type_hints,
                expected,
                resolver_policy_stamp: request.policy_stamp.resolver.clone(),
            })
        })
        .collect()
}

/// Resolve every resource a parsed transform graph may read against the
/// request's concrete `resourceVersions` manifest. Exact references and
/// path/binding patterns never trigger host-side listing.
pub fn missing_command_graph_resource_reads_v1(
    request: &CommandServiceRequestV1,
    limits: CommandServiceLimitsV1,
) -> Result<Vec<CommandResourceReadRequestV1>, CommandResourceHydrationErrorV1> {
    validate_command_service_request_v1(request, limits)?;
    let PortableOperationRequestV1::Transform {
        source: CommandTransformSourceV1::Graph { config_uri },
        ..
    } = &request.operation
    else {
        return Ok(Vec::new());
    };
    let config = request.resources.get(config_uri).ok_or_else(|| {
        CommandResourceHydrationErrorV1::GraphResourceManifest {
            uri: config_uri.clone(),
            message: "the graph config must be hydrated before dependency matching".to_owned(),
        }
    })?;
    let mut identity = config.identity.clone().unwrap_or_default();
    if identity.content_type.is_none() {
        identity.content_type = Some("application/cem+xml".to_owned());
    }
    if identity.schema.is_none() {
        identity.schema = Some(crate::transform_config::TRANSFORM_CONFIG_SCHEMA_URI.to_owned());
    }
    identity.base_uri = Some(config_uri.clone());
    let parsed = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: config.bytes.clone(),
        identity,
        base_uri: Some(config_uri.clone()),
    })
    .map_err(
        |error| CommandResourceHydrationErrorV1::GraphResourceManifest {
            uri: config_uri.clone(),
            message: format!("{}: {}", error.code, error.message),
        },
    )?;
    if !parsed.diagnostics.is_empty() {
        return Err(CommandResourceHydrationErrorV1::GraphResourceManifest {
            uri: config_uri.clone(),
            message: parsed
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    let mut needs = BTreeMap::<String, ResourceNeed>::new();
    for node in &parsed.graph.nodes {
        match node.kind {
            TransformGraphNodeKind::Import => {
                if let Some(reference) = node.src.as_deref() {
                    add_graph_reference_needs(
                        request,
                        &mut needs,
                        config_uri,
                        &node.id,
                        "src",
                        reference,
                        ResolvePurpose::Input,
                        node.content_type.as_deref(),
                    )?;
                }
            }
            TransformGraphNodeKind::Transform => {
                if let Some(reference) = node.src.as_deref() {
                    add_graph_reference_needs(
                        request,
                        &mut needs,
                        config_uri,
                        &node.id,
                        "src",
                        reference,
                        ResolvePurpose::Template,
                        node.template_content_type.as_deref(),
                    )?;
                }
            }
            TransformGraphNodeKind::ImportMapRewrite => {
                for (field, reference) in [
                    ("sourceMap", node.source_map.as_deref()),
                    ("targetMap", node.target_map.as_deref()),
                ] {
                    if let Some(reference) = reference {
                        add_graph_reference_needs(
                            request,
                            &mut needs,
                            config_uri,
                            &node.id,
                            field,
                            reference,
                            ResolvePurpose::ModuleMap,
                            Some("application/json"),
                        )?;
                    }
                }
            }
            TransformGraphNodeKind::Join | TransformGraphNodeKind::Export => {}
        }
    }

    needs
        .into_iter()
        .map(|(uri, need)| {
            let expected = request
                .resource_versions
                .get(&uri)
                .cloned()
                .expect("graph need was selected from the concrete resource manifest");
            Ok(CommandResourceReadRequestV1 {
                request_id: request.request_id.clone(),
                project: request.project.clone(),
                uri,
                purposes: need.purposes,
                content_type_hints: need.content_type_hints,
                expected,
                resolver_policy_stamp: request.policy_stamp.resolver.clone(),
            })
        })
        .collect()
}

/// Admit and hydrate a request without exposing live callbacks in its wire
/// representation. Reads run sequentially in URI order so hosts observe a
/// deterministic, bounded request sequence.
pub async fn hydrate_command_service_request_v1(
    request: &CommandServiceRequestV1,
    ledger: &CommandRevisionLedgerV1,
    reader: &dyn CommandResourceReaderV1,
    limits: CommandServiceLimitsV1,
) -> Result<CommandServiceHydrationV1, CommandResourceHydrationErrorV1> {
    hydrate_command_service_request_inner_v1(request, ledger, reader, limits, None).await
}

/// Hydrate an admitted operation while checking its root control before and
/// after every asynchronous host read. This keeps cancellation cooperative
/// without serializing a signal into the command-service request.
pub async fn hydrate_command_service_operation_v1(
    request: &CommandServiceRequestV1,
    ledger: &CommandRevisionLedgerV1,
    reader: &dyn CommandResourceReaderV1,
    limits: CommandServiceLimitsV1,
    control: &OperationControl,
) -> Result<CommandServiceHydrationV1, CommandResourceHydrationErrorV1> {
    hydrate_command_service_request_inner_v1(request, ledger, reader, limits, Some(control)).await
}

async fn hydrate_command_service_request_inner_v1(
    request: &CommandServiceRequestV1,
    ledger: &CommandRevisionLedgerV1,
    reader: &dyn CommandResourceReaderV1,
    limits: CommandServiceLimitsV1,
    control: Option<&OperationControl>,
) -> Result<CommandServiceHydrationV1, CommandResourceHydrationErrorV1> {
    match admit_command_service_request_v1(request, ledger, limits)? {
        CommandServiceAdmissionV1::Stale(stale) => {
            return Ok(CommandServiceHydrationV1::Stale(stale));
        }
        CommandServiceAdmissionV1::Accepted => {}
    }

    let mut hydrated = request.clone();
    hydrate_command_reads_v1(
        &mut hydrated,
        missing_command_resource_reads_v1(request, limits)?,
        reader,
        control,
    )
    .await?;
    let graph_reads = missing_command_graph_resource_reads_v1(&hydrated, limits)?;
    hydrate_command_reads_v1(&mut hydrated, graph_reads, reader, control).await?;
    check_hydration_control(control)?;
    validate_command_service_request_v1(&hydrated, limits)?;
    Ok(CommandServiceHydrationV1::Ready(Box::new(hydrated)))
}

async fn hydrate_command_reads_v1(
    hydrated: &mut CommandServiceRequestV1,
    reads: Vec<CommandResourceReadRequestV1>,
    reader: &dyn CommandResourceReaderV1,
    control: Option<&OperationControl>,
) -> Result<(), CommandResourceHydrationErrorV1> {
    for read_request in reads {
        check_hydration_control(control)?;
        let uri = read_request.uri.clone();
        let expected = read_request.expected.clone();
        let resolved = reader.read(read_request).await.map_err(|source| {
            CommandResourceHydrationErrorV1::HostRead {
                uri: uri.clone(),
                source,
            }
        })?;
        check_hydration_control(control)?;
        if resolved.version.revision != expected.revision {
            return Err(CommandResourceHydrationErrorV1::ResourceRevisionMismatch {
                uri,
                expected: expected.revision,
                actual: resolved.version.revision,
            });
        }
        if resolved.version.sha256 != expected.sha256 {
            return Err(CommandResourceHydrationErrorV1::ResourceDigestMismatch {
                uri,
                expected: expected.sha256,
                actual: resolved.version.sha256,
            });
        }
        let actual = sha256_hex(&resolved.bytes);
        if actual != expected.sha256 {
            return Err(
                CommandResourceHydrationErrorV1::ResourceBytesDigestMismatch {
                    uri,
                    expected: expected.sha256,
                    actual,
                },
            );
        }
        hydrated.resources.insert(
            uri,
            VirtualResourceV1 {
                bytes: resolved.bytes,
                identity: resolved.identity,
            },
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_graph_reference_needs(
    request: &CommandServiceRequestV1,
    needs: &mut BTreeMap<String, ResourceNeed>,
    config_uri: &str,
    node_id: &str,
    field: &str,
    reference: &str,
    purpose: ResolvePurpose,
    content_type_hint: Option<&str>,
) -> Result<(), CommandResourceHydrationErrorV1> {
    let resolved = resolve_graph_reference(config_uri, reference);
    let patterned = resolved.contains('*') || resolved.contains('?') || resolved.contains('{');
    let matches = if patterned {
        let matcher = graph_reference_matcher(&resolved).map_err(|message| {
            CommandResourceHydrationErrorV1::GraphResourceManifest {
                uri: config_uri.to_owned(),
                message: format!("node `{node_id}` field `{field}`: {message}"),
            }
        })?;
        request
            .resource_versions
            .keys()
            .filter(|uri| matcher.is_match(uri))
            .cloned()
            .collect::<Vec<_>>()
    } else if request.resource_versions.contains_key(&resolved) {
        vec![resolved.clone()]
    } else {
        Vec::new()
    };
    if matches.is_empty() {
        return Err(CommandResourceHydrationErrorV1::GraphResourceManifest {
            uri: config_uri.to_owned(),
            message: format!(
                "node `{node_id}` field `{field}` reference `{reference}` matched no concrete resourceVersions URI"
            ),
        });
    }
    for uri in matches {
        add_need(request, needs, &uri, purpose, content_type_hint);
    }
    Ok(())
}

fn resolve_graph_reference(config_uri: &str, reference: &str) -> String {
    if crate::resolver::uri_scheme(reference).is_some() || reference.starts_with('/') {
        return reference.to_owned();
    }
    let base = config_uri
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or(config_uri);
    normalize_graph_uri(&format!("{base}/{reference}"))
}

fn normalize_graph_uri(uri: &str) -> String {
    let (prefix, path) = if let Some(scheme) = uri.find("://") {
        let authority_start = scheme + 3;
        match uri[authority_start..].find('/') {
            Some(path_start) => {
                let path_start = authority_start + path_start;
                (&uri[..path_start], &uri[path_start + 1..])
            }
            None => (uri, ""),
        }
    } else if let Some(path) = uri.strip_prefix('/') {
        ("", path)
    } else {
        ("", uri)
    };
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }
    if prefix.is_empty() {
        segments.join("/")
    } else {
        format!("{prefix}/{}", segments.join("/"))
    }
}

fn graph_reference_matcher(pattern: &str) -> Result<Regex, String> {
    let mut expression = String::from("^");
    let mut characters = pattern.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '*' if characters.peek() == Some(&'*') => {
                characters.next();
                expression.push_str(".*");
            }
            '*' => expression.push_str("[^/]*"),
            '?' => expression.push_str("[^/]"),
            '{' => {
                let mut name = String::new();
                loop {
                    match characters.next() {
                        Some('}') => break,
                        Some(character) => name.push(character),
                        None => {
                            return Err(format!(
                                "reference pattern `{pattern}` has an unclosed binding"
                            ))
                        }
                    }
                }
                if name.trim().is_empty() {
                    return Err(format!(
                        "reference pattern `{pattern}` has an empty binding"
                    ));
                }
                expression.push_str("[^/]+");
            }
            '}' => {
                return Err(format!(
                    "reference pattern `{pattern}` has an unmatched closing binding"
                ))
            }
            '[' | ']' => {
                return Err(format!(
                    "reference pattern `{pattern}` uses unsupported character-class syntax"
                ))
            }
            _ => expression.push_str(&regex::escape(&character.to_string())),
        }
    }
    expression.push('$');
    Regex::new(&expression).map_err(|error| error.to_string())
}

fn check_hydration_control(
    control: Option<&OperationControl>,
) -> Result<(), CommandResourceHydrationErrorV1> {
    match control {
        Some(control) => control
            .check_scope(control.root_scope())
            .map_err(CommandResourceHydrationErrorV1::Control),
        None => Ok(()),
    }
}

fn add_input_need(
    request: &CommandServiceRequestV1,
    plan: &NormalizedRunPlan,
    needs: &mut BTreeMap<String, ResourceNeed>,
    input_id: &str,
) -> Result<(), CommandResourceHydrationErrorV1> {
    let input = plan
        .inputs
        .iter()
        .find(|input| input.input_id == input_id)
        .ok_or_else(|| CommandServiceError::UnknownInputId {
            input_id: input_id.to_owned(),
        })?;
    add_need(
        request,
        needs,
        input.resolved_uri.as_deref().unwrap_or(&input.declared_uri),
        ResolvePurpose::Input,
        input.identity.content_type.as_deref(),
    );
    Ok(())
}

fn query_content_type_hint(uri: &str) -> Option<String> {
    infer_content_type_from_path(uri).or_else(|| {
        uri.to_ascii_lowercase()
            .ends_with(".xpath")
            .then(|| XPATH_CONTENT_TYPE.to_owned())
    })
}

fn add_need(
    request: &CommandServiceRequestV1,
    needs: &mut BTreeMap<String, ResourceNeed>,
    uri: &str,
    purpose: ResolvePurpose,
    content_type_hint: Option<&str>,
) {
    if request.resources.contains_key(uri) {
        return;
    }
    let need = needs.entry(uri.to_owned()).or_default();
    need.purposes.insert(purpose);
    if let Some(content_type_hint) = content_type_hint {
        need.content_type_hints.insert(content_type_hint.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Wake, Waker};

    use super::*;
    use crate::capability::{capability_manifest, CapabilityRequest, RuntimeKind};
    use crate::command_operation::{prepare_command_operation_v1, PreparedPortableOperationV1};
    use crate::command_service::{
        CommandPolicyStampV1, CommandRunPlanV1, CommandUriMapV1, COMMAND_SERVICE_PROTOCOL_VERSION,
    };
    use crate::engine::{EngineContext, ParseProjection, TransformTemplateEntrypoint};
    use crate::run_config::{
        parse_normalized_run_plan, NormalizedRunPlanRequest, RunConfigDefaults,
    };

    const DATA_URI: &str = "studio://catalog/a-data.cem";
    const SECOND_URI: &str = "studio://catalog/b-data.cem";
    const QUERY_URI: &str = "studio://catalog/query.xpath";
    const GRAPH_URI: &str = "studio://catalog/graph.cem";
    const GRAPH_INPUT_A_URI: &str = "studio://catalog/inputs/a.cem";
    const GRAPH_INPUT_B_URI: &str = "studio://catalog/inputs/b.cem";
    const GRAPH_TEMPLATE_URI: &str = "studio://catalog/templates/main.cem";
    const GRAPH_BYTES: &[u8] = br#"{@doc cem-ml 1}{run | {import @id=data @src="inputs/*.cem" @content-type="application/cem+xml" | {transform @id=view @src="templates/main.cem" @template-content-type="text/cem-ml"}}}"#;

    #[derive(Default)]
    struct FixtureReader {
        responses:
            BTreeMap<String, Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>>,
        calls: Mutex<Vec<CommandResourceReadRequestV1>>,
    }

    impl FixtureReader {
        fn with_resource(mut self, uri: &str, bytes: &[u8]) -> Self {
            self.responses.insert(
                uri.to_owned(),
                Ok(CommandResolvedResourceV1 {
                    version: version(bytes),
                    bytes: bytes.to_vec(),
                    identity: None,
                }),
            );
            self
        }

        fn with_response(
            mut self,
            uri: &str,
            response: Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>,
        ) -> Self {
            self.responses.insert(uri.to_owned(), response);
            self
        }

        fn calls(&self) -> Vec<CommandResourceReadRequestV1> {
            self.calls.lock().expect("fixture calls mutex").clone()
        }
    }

    impl CommandResourceReaderV1 for FixtureReader {
        fn read<'a>(
            &'a self,
            request: CommandResourceReadRequestV1,
        ) -> CommandHostFuture<'a, Result<CommandResolvedResourceV1, CommandResourceReadFailureV1>>
        {
            self.calls
                .lock()
                .expect("fixture calls mutex")
                .push(request.clone());
            let response = self
                .responses
                .get(&request.uri)
                .cloned()
                .unwrap_or_else(|| {
                    Err(CommandResourceReadFailureV1::new(
                        "fixture.missing",
                        "fixture has no resource response",
                    ))
                });
            Box::pin(std::future::ready(response))
        }
    }

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn plan() -> NormalizedRunPlan {
        parse_normalized_run_plan(NormalizedRunPlanRequest {
            input_records: vec![
                format!("uri={DATA_URI},contentType=application/cem+xml"),
                format!("uri={SECOND_URI},contentType=application/cem+xml"),
            ],
            defaults: RunConfigDefaults::default(),
            ..NormalizedRunPlanRequest::default()
        })
        .expect("normalized hydration fixture plan")
    }

    fn version(bytes: &[u8]) -> CommandResourceVersionV1 {
        CommandResourceVersionV1 {
            revision: 1,
            sha256: sha256_hex(bytes),
        }
    }

    fn request(operation: PortableOperationRequestV1) -> CommandServiceRequestV1 {
        let mut resource_versions = CommandUriMapV1::from(BTreeMap::from([
            (DATA_URI.to_owned(), version(b"<catalog/>")),
            (SECOND_URI.to_owned(), version(b"<second/>")),
        ]));
        match &operation {
            PortableOperationRequestV1::Query { query_uri, .. } => {
                resource_versions.insert(query_uri.clone(), version(b"//item"));
            }
            PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Direct { template_uri, .. },
                ..
            } => {
                resource_versions
                    .entry(template_uri.clone())
                    .or_insert_with(|| version(b"{@doc cem-ml 1}{template @name=main}"));
            }
            PortableOperationRequestV1::Transform {
                source: CommandTransformSourceV1::Graph { config_uri },
                ..
            } => {
                resource_versions
                    .entry(config_uri.clone())
                    .or_insert_with(|| version(GRAPH_BYTES));
            }
            _ => {}
        }
        CommandServiceRequestV1 {
            protocol_version: COMMAND_SERVICE_PROTOCOL_VERSION,
            request_id: "request:hydrate".to_owned(),
            project: CommandProjectRevisionV1 {
                project_id: "catalog".to_owned(),
                revision: 7,
            },
            resource_versions,
            operation,
            run_plan: CommandRunPlanV1::from(plan()),
            resources: CommandUriMapV1::new(),
            policy_stamp: CommandPolicyStampV1 {
                resolver: "resolver:fixture".to_owned(),
                safety: "safety:fixture".to_owned(),
                budget: "budget:fixture".to_owned(),
            },
        }
    }

    fn ledger(request: &CommandServiceRequestV1) -> CommandRevisionLedgerV1 {
        CommandRevisionLedgerV1 {
            project: request.project.clone(),
            resource_versions: request.resource_versions.clone(),
        }
    }

    #[test]
    fn missing_reads_are_uri_sorted_aggregate_purposes_and_bypass_inline_resources() {
        let validate = request(PortableOperationRequestV1::Validate {
            input_ids: vec!["input:1".to_owned(), "input:0".to_owned()],
            projection: crate::engine::ValidateProjection::Json,
        });
        let reads = missing_command_resource_reads_v1(&validate, CommandServiceLimitsV1::default())
            .expect("missing validate reads");
        assert_eq!(
            reads
                .iter()
                .map(|read| read.uri.as_str())
                .collect::<Vec<_>>(),
            [DATA_URI, SECOND_URI]
        );
        assert!(reads
            .iter()
            .all(|read| read.purposes == BTreeSet::from([ResolvePurpose::Input])));

        let mut shared = request(PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Direct {
                data_input_id: "input:0".to_owned(),
                template_uri: DATA_URI.to_owned(),
            },
            params: BTreeMap::new(),
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            preserve_source_offsets: false,
        });
        let reads = missing_command_resource_reads_v1(&shared, CommandServiceLimitsV1::default())
            .expect("shared resource read");
        assert_eq!(reads.len(), 1);
        assert_eq!(
            reads[0].purposes,
            BTreeSet::from([ResolvePurpose::Input, ResolvePurpose::Template])
        );
        shared.resources.insert(
            DATA_URI.to_owned(),
            VirtualResourceV1 {
                bytes: b"<catalog/>".to_vec(),
                identity: None,
            },
        );
        assert!(
            missing_command_resource_reads_v1(&shared, CommandServiceLimitsV1::default())
                .expect("inline resource bypass")
                .is_empty()
        );
    }

    #[test]
    fn hydration_stops_at_stale_admission_without_calling_the_host() {
        let request = request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: false,
        });
        let mut stale = ledger(&request);
        stale.project.revision += 1;
        let reader = FixtureReader::default().with_resource(DATA_URI, b"<catalog/>");

        let hydrated = block_on(hydrate_command_service_request_v1(
            &request,
            &stale,
            &reader,
            CommandServiceLimitsV1::default(),
        ))
        .expect("stale admission is an outcome");
        let CommandServiceHydrationV1::Stale(stale) = hydrated else {
            panic!("expected stale hydration outcome")
        };
        assert_eq!(stale.current_project_revision, 8);
        assert!(reader.calls().is_empty());
    }

    #[test]
    fn hydration_rejects_host_failure_revision_digest_and_byte_drift() {
        let request = request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::Json,
            preserve_source_offsets: false,
        });
        let ledger = ledger(&request);

        let host_failure = FixtureReader::default().with_response(
            DATA_URI,
            Err(CommandResourceReadFailureV1::new(
                "fixture.denied",
                "read denied",
            )),
        );
        let error = block_on(hydrate_command_service_request_v1(
            &request,
            &ledger,
            &host_failure,
            CommandServiceLimitsV1::default(),
        ))
        .expect_err("host failure rejects hydration");
        assert_eq!(error.code(), "cem.command_service.host_read");

        let mut revision = version(b"<catalog/>");
        revision.revision = 2;
        let revision_drift = FixtureReader::default().with_response(
            DATA_URI,
            Ok(CommandResolvedResourceV1 {
                version: revision,
                bytes: b"<catalog/>".to_vec(),
                identity: None,
            }),
        );
        let error = block_on(hydrate_command_service_request_v1(
            &request,
            &ledger,
            &revision_drift,
            CommandServiceLimitsV1::default(),
        ))
        .expect_err("revision drift rejects hydration");
        assert_eq!(
            error.code(),
            "cem.command_service.host_resource_revision_mismatch"
        );

        let digest_drift = FixtureReader::default().with_response(
            DATA_URI,
            Ok(CommandResolvedResourceV1 {
                version: version(b"other"),
                bytes: b"other".to_vec(),
                identity: None,
            }),
        );
        let error = block_on(hydrate_command_service_request_v1(
            &request,
            &ledger,
            &digest_drift,
            CommandServiceLimitsV1::default(),
        ))
        .expect_err("declared digest drift rejects hydration");
        assert_eq!(
            error.code(),
            "cem.command_service.host_resource_digest_mismatch"
        );

        let byte_drift = FixtureReader::default().with_response(
            DATA_URI,
            Ok(CommandResolvedResourceV1 {
                version: version(b"<catalog/>"),
                bytes: b"changed".to_vec(),
                identity: None,
            }),
        );
        let error = block_on(hydrate_command_service_request_v1(
            &request,
            &ledger,
            &byte_drift,
            CommandServiceLimitsV1::default(),
        ))
        .expect_err("byte digest drift rejects hydration");
        assert_eq!(
            error.code(),
            "cem.command_service.host_resource_bytes_digest_mismatch"
        );
    }

    #[test]
    fn hydrated_request_hands_off_to_owned_operation_preparation() {
        let mut request = request(PortableOperationRequestV1::Parse {
            input_id: "input:0".to_owned(),
            projection: ParseProjection::DomJson,
            preserve_source_offsets: true,
        });
        request.resources.insert(
            SECOND_URI.to_owned(),
            VirtualResourceV1 {
                bytes: b"<second/>".to_vec(),
                identity: None,
            },
        );
        let ledger = ledger(&request);
        let reader = FixtureReader::default().with_resource(DATA_URI, b"<catalog/>");

        let hydrated = block_on(hydrate_command_service_request_v1(
            &request,
            &ledger,
            &reader,
            CommandServiceLimitsV1::default(),
        ))
        .expect("request hydration");
        let CommandServiceHydrationV1::Ready(hydrated) = hydrated else {
            panic!("expected ready hydration outcome")
        };
        assert_eq!(reader.calls().len(), 1);
        assert_eq!(reader.calls()[0].uri, DATA_URI);
        assert_eq!(reader.calls()[0].resolver_policy_stamp, "resolver:fixture");
        assert!(hydrated.resources.contains_key(DATA_URI));
        assert!(hydrated.resources.contains_key(SECOND_URI));

        let capability = capability_manifest(CapabilityRequest {
            runtime: RuntimeKind::Native,
            target_identity: "x86_64-unknown-linux-gnu".to_owned(),
            abi_identity: "rust:1".to_owned(),
            debug_control_active: false,
        })
        .expect("fixture capability");
        let prepared = prepare_command_operation_v1(
            &hydrated,
            CommandServiceLimitsV1::default(),
            &EngineContext::default(),
            &capability,
        )
        .expect("hydrated request prepares");
        let PreparedPortableOperationV1::Parse(prepared) = prepared else {
            panic!("prepared parse operation")
        };
        assert_eq!(prepared.input.bytes, b"<catalog/>");
        assert!(prepared.preserve_source_offsets);
    }

    #[test]
    fn query_read_carries_query_purpose_and_inferred_content_type() {
        let request = request(PortableOperationRequestV1::Query {
            data_input_id: "input:0".to_owned(),
            query_uri: QUERY_URI.to_owned(),
            output: crate::query::QueryExportFormat::Json,
        });
        let reads = missing_command_resource_reads_v1(&request, CommandServiceLimitsV1::default())
            .expect("query reads");
        let query = reads
            .iter()
            .find(|read| read.uri == QUERY_URI)
            .expect("query read request");
        assert_eq!(query.purposes, BTreeSet::from([ResolvePurpose::Query]));
        assert_eq!(
            query.content_type_hints,
            BTreeSet::from([XPATH_CONTENT_TYPE.to_owned()])
        );
    }

    #[test]
    fn graph_manifest_matches_concrete_resources_and_hydrates_without_host_listing() {
        let mut request = request(PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Graph {
                config_uri: GRAPH_URI.to_owned(),
            },
            params: BTreeMap::new(),
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            preserve_source_offsets: true,
        });
        for (uri, bytes) in [
            (GRAPH_INPUT_B_URI, b"{b}".as_slice()),
            (GRAPH_INPUT_A_URI, b"{a}".as_slice()),
            (GRAPH_TEMPLATE_URI, b"{template}".as_slice()),
        ] {
            request
                .resource_versions
                .insert(uri.to_owned(), version(bytes));
        }
        request.resources.insert(
            GRAPH_URI.to_owned(),
            VirtualResourceV1 {
                bytes: GRAPH_BYTES.to_vec(),
                identity: None,
            },
        );

        let reads =
            missing_command_graph_resource_reads_v1(&request, CommandServiceLimitsV1::default())
                .expect("graph manifest resolves");
        assert_eq!(
            reads
                .iter()
                .map(|read| read.uri.as_str())
                .collect::<Vec<_>>(),
            [GRAPH_INPUT_A_URI, GRAPH_INPUT_B_URI, GRAPH_TEMPLATE_URI]
        );
        assert_eq!(reads[0].purposes, BTreeSet::from([ResolvePurpose::Input]));
        assert_eq!(
            reads[2].purposes,
            BTreeSet::from([ResolvePurpose::Template])
        );

        request.resources.remove(GRAPH_URI);
        let reader = FixtureReader::default()
            .with_resource(GRAPH_URI, GRAPH_BYTES)
            .with_resource(GRAPH_INPUT_A_URI, b"{a}")
            .with_resource(GRAPH_INPUT_B_URI, b"{b}")
            .with_resource(GRAPH_TEMPLATE_URI, b"{template}");
        let hydrated = block_on(hydrate_command_service_request_v1(
            &request,
            &ledger(&request),
            &reader,
            CommandServiceLimitsV1::default(),
        ))
        .expect("two-phase graph hydration");
        assert!(matches!(hydrated, CommandServiceHydrationV1::Ready(_)));
        assert_eq!(
            reader
                .calls()
                .iter()
                .map(|read| read.uri.as_str())
                .collect::<Vec<_>>(),
            [
                GRAPH_URI,
                GRAPH_INPUT_A_URI,
                GRAPH_INPUT_B_URI,
                GRAPH_TEMPLATE_URI
            ]
        );
    }

    #[test]
    fn graph_manifest_rejects_unmatched_and_malformed_patterns() {
        let mut request = request(PortableOperationRequestV1::Transform {
            source: CommandTransformSourceV1::Graph {
                config_uri: GRAPH_URI.to_owned(),
            },
            params: BTreeMap::new(),
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            preserve_source_offsets: false,
        });
        request.resources.insert(
            GRAPH_URI.to_owned(),
            VirtualResourceV1 {
                bytes: GRAPH_BYTES.to_vec(),
                identity: None,
            },
        );
        let error =
            missing_command_graph_resource_reads_v1(&request, CommandServiceLimitsV1::default())
                .expect_err("unmatched graph resources reject");
        assert_eq!(error.code(), "cem.command_service.graph_resource_manifest");
        assert!(error.to_string().contains("matched no concrete"));

        let error = graph_reference_matcher("studio://catalog/{stem.json")
            .expect_err("unclosed binding rejects");
        assert!(error.contains("unclosed binding"));
    }
}
