//! Schema-owned content conversion registry.
//!
//! This module is the planning and dispatch-side contract for schema package
//! converter edges. Runtime execution still flows through the existing
//! lifecycle and transform-template adapters.

use crate::engine::FormatIdentity;
use crate::schema::registry::{
    content_type_essence, SchemaContentTypeRole, SchemaDescriptor, SchemaRegistry,
    CEM_AST_JSON_PROJECTION_CONTENT_TYPE, CEM_AST_PROJECTION_CONTENT_TYPE,
    CEM_AST_PROJECTION_SCHEMA_URI, CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
    CEM_DOM_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
    CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_CONTENT_TYPE,
    CEM_EVENTS_PROJECTION_SCHEMA_URI, CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI,
    CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, HTML_CONTENT_TYPE, HTML_SCHEMA_URI,
    XML_CONTENT_TYPE, XML_SCHEMA_URI,
};
use crate::transform_template::{
    TransformTemplateAdapterCapability, TransformTemplateAdapterLookup,
    TransformTemplateAdapterRegistry,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConversionImplementation {
    Cemt,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionReadiness {
    Ready,
    Planned,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConversionEndpoint {
    pub content_type: String,
    pub schema: Option<String>,
}

impl ConversionEndpoint {
    pub fn new(content_type: impl Into<String>) -> Self {
        Self {
            content_type: content_type_essence(&content_type.into()),
            schema: None,
        }
    }

    pub fn with_schema(content_type: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            content_type: content_type_essence(&content_type.into()),
            schema: Some(schema.into()),
        }
    }

    fn matches(&self, identity: &ResolvedConversionIdentity) -> bool {
        self.content_type == identity.content_type
            && self
                .schema
                .as_deref()
                .map(|schema| schema == identity.schema)
                .unwrap_or(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionTemplateDescriptor {
    pub path: String,
    pub content_type: String,
    pub schema: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionRustFallbackDescriptor {
    pub rust_symbol: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionDescriptor {
    pub id: String,
    pub package_id: String,
    pub from: ConversionEndpoint,
    pub to: ConversionEndpoint,
    pub implementation: ConversionImplementation,
    pub readiness: ConversionReadiness,
    pub template: Option<ConversionTemplateDescriptor>,
    pub rust_symbol: Option<String>,
    pub rust_fallback: Option<ConversionRustFallbackDescriptor>,
    pub streamable: bool,
    pub lossiness: Option<String>,
    pub implicit: bool,
    pub explicit_only: bool,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConversionIdentity {
    pub content_type: String,
    pub schema: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionIdentityError {
    EmptyIdentity,
    UnknownContentType {
        content_type: String,
    },
    AmbiguousContentType {
        content_type: String,
        schema_uris: Vec<String>,
    },
    SchemaMismatch {
        content_type: String,
        schema: String,
        candidate_schemas: Vec<String>,
    },
    UnknownSchema {
        schema: String,
    },
    SchemaHasNoPrimaryContentType {
        schema: String,
    },
    UnknownNamespace {
        namespaces: Vec<String>,
    },
    AmbiguousNamespace {
        namespaces: Vec<String>,
        schema_uris: Vec<String>,
    },
}

impl std::fmt::Display for ConversionIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentity => write!(f, "conversion identity is empty"),
            Self::UnknownContentType { content_type } => {
                write!(f, "unknown conversion content type `{content_type}`")
            }
            Self::AmbiguousContentType {
                content_type,
                schema_uris,
            } => write!(
                f,
                "conversion content type `{content_type}` is ambiguous across schemas: {}",
                schema_uris.join(", ")
            ),
            Self::SchemaMismatch {
                content_type,
                schema,
                candidate_schemas,
            } => write!(
                f,
                "conversion content type `{content_type}` is not owned by schema `{schema}`; candidates: {}",
                candidate_schemas.join(", ")
            ),
            Self::UnknownSchema { schema } => {
                write!(f, "unknown conversion schema `{schema}`")
            }
            Self::SchemaHasNoPrimaryContentType { schema } => write!(
                f,
                "conversion schema `{schema}` has no primary content type"
            ),
            Self::UnknownNamespace { namespaces } => write!(
                f,
                "no conversion schema matched namespaces: {}",
                namespaces.join(", ")
            ),
            Self::AmbiguousNamespace {
                namespaces,
                schema_uris,
            } => write!(
                f,
                "conversion namespaces {} are ambiguous across schemas: {}",
                namespaces.join(", "),
                schema_uris.join(", ")
            ),
        }
    }
}

impl std::error::Error for ConversionIdentityError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionRegistryError {
    DuplicateConverterId { id: String },
}

impl std::fmt::Display for ConversionRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateConverterId { id } => {
                write!(f, "converter `{id}` is already registered")
            }
        }
    }
}

impl std::error::Error for ConversionRegistryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionLookupError {
    SourceIdentity(ConversionIdentityError),
    TargetIdentity(ConversionIdentityError),
    NoDirectEdge {
        source: ResolvedConversionIdentity,
        target: ResolvedConversionIdentity,
    },
    AmbiguousDirectEdge {
        source: ResolvedConversionIdentity,
        target: ResolvedConversionIdentity,
        edge_ids: Vec<String>,
    },
}

impl std::fmt::Display for ConversionLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceIdentity(error) => write!(f, "invalid source identity: {error}"),
            Self::TargetIdentity(error) => write!(f, "invalid target identity: {error}"),
            Self::NoDirectEdge { source, target } => write!(
                f,
                "no direct conversion edge from `{}` ({}) to `{}` ({})",
                source.content_type, source.schema, target.content_type, target.schema
            ),
            Self::AmbiguousDirectEdge { edge_ids, .. } => {
                write!(
                    f,
                    "direct conversion edge is ambiguous: {}",
                    edge_ids.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for ConversionLookupError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionLookupOptions {
    pub include_explicit_only: bool,
}

impl ConversionLookupOptions {
    pub fn implicit() -> Self {
        Self {
            include_explicit_only: false,
        }
    }

    pub fn explicit() -> Self {
        Self {
            include_explicit_only: true,
        }
    }
}

impl Default for ConversionLookupOptions {
    fn default() -> Self {
        Self::implicit()
    }
}

#[derive(Debug, Clone)]
pub struct DirectConversionSelection<'a> {
    pub source: ResolvedConversionIdentity,
    pub target: ResolvedConversionIdentity,
    pub descriptor: &'a ConversionDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionExecution {
    CemtTemplate {
        adapter_id: &'static str,
        template: ConversionTemplateDescriptor,
    },
    Rust {
        rust_symbol: String,
    },
    RustFallback {
        rust_symbol: String,
        reason: String,
        template_adapter_id: Option<&'static str>,
    },
}

#[derive(Debug, Clone)]
pub struct DirectConversionExecution<'a> {
    pub source: ResolvedConversionIdentity,
    pub target: ResolvedConversionIdentity,
    pub descriptor: &'a ConversionDescriptor,
    pub execution: ConversionExecution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionExecutionError {
    Lookup(ConversionLookupError),
    MissingTemplate {
        converter_id: String,
    },
    MissingRustSymbol {
        converter_id: String,
    },
    CemtExecutionUnavailable {
        converter_id: String,
        reason: String,
    },
}

impl std::fmt::Display for ConversionExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lookup(error) => write!(f, "{error}"),
            Self::MissingTemplate { converter_id } => {
                write!(
                    f,
                    "CEMT converter `{converter_id}` has no template descriptor"
                )
            }
            Self::MissingRustSymbol { converter_id } => {
                write!(f, "Rust converter `{converter_id}` has no rust symbol")
            }
            Self::CemtExecutionUnavailable {
                converter_id,
                reason,
            } => write!(
                f,
                "CEMT converter `{converter_id}` cannot be executed: {reason}"
            ),
        }
    }
}

impl std::error::Error for ConversionExecutionError {}

impl From<ConversionLookupError> for ConversionExecutionError {
    fn from(error: ConversionLookupError) -> Self {
        Self::Lookup(error)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConversionRegistry {
    descriptors_by_id: BTreeMap<String, ConversionDescriptor>,
}

impl ConversionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_converters() -> Self {
        let mut registry = Self::new();
        for descriptor in builtin_conversion_descriptors() {
            registry
                .register(descriptor)
                .expect("built-in conversion descriptors must not conflict");
        }
        registry
    }

    pub fn register(
        &mut self,
        descriptor: ConversionDescriptor,
    ) -> Result<(), ConversionRegistryError> {
        if self.descriptors_by_id.contains_key(&descriptor.id) {
            return Err(ConversionRegistryError::DuplicateConverterId { id: descriptor.id });
        }
        self.descriptors_by_id
            .insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    pub fn converter(&self, id: &str) -> Option<&ConversionDescriptor> {
        self.descriptors_by_id.get(id)
    }

    pub fn converters(&self) -> impl Iterator<Item = &ConversionDescriptor> {
        self.descriptors_by_id.values()
    }

    pub fn select_direct_edge<'a>(
        &'a self,
        schema_registry: &SchemaRegistry,
        source: &FormatIdentity,
        target: &FormatIdentity,
    ) -> Result<DirectConversionSelection<'a>, ConversionLookupError> {
        self.select_direct_edge_with_options(
            schema_registry,
            source,
            target,
            ConversionLookupOptions::default(),
        )
    }

    pub fn select_direct_edge_with_options<'a>(
        &'a self,
        schema_registry: &SchemaRegistry,
        source: &FormatIdentity,
        target: &FormatIdentity,
        options: ConversionLookupOptions,
    ) -> Result<DirectConversionSelection<'a>, ConversionLookupError> {
        let source = resolve_identity(source, schema_registry)
            .map_err(ConversionLookupError::SourceIdentity)?;
        let target = resolve_identity(target, schema_registry)
            .map_err(ConversionLookupError::TargetIdentity)?;

        let candidates = self
            .descriptors_by_id
            .values()
            .filter(|descriptor| descriptor_can_plan(descriptor, options))
            .filter(|descriptor| descriptor.from.matches(&source) && descriptor.to.matches(&target))
            .collect::<Vec<_>>();

        let Some(best_rank) = candidates
            .iter()
            .map(|descriptor| descriptor_rank(descriptor))
            .min()
        else {
            return Err(ConversionLookupError::NoDirectEdge { source, target });
        };

        let mut best = candidates
            .into_iter()
            .filter(|descriptor| descriptor_rank(descriptor) == best_rank)
            .collect::<Vec<_>>();
        best.sort_by(|a, b| a.id.cmp(&b.id));

        match best.as_slice() {
            [descriptor] => Ok(DirectConversionSelection {
                source,
                target,
                descriptor,
            }),
            descriptors => Err(ConversionLookupError::AmbiguousDirectEdge {
                source,
                target,
                edge_ids: descriptors
                    .iter()
                    .map(|descriptor| descriptor.id.clone())
                    .collect(),
            }),
        }
    }

    pub fn resolve_direct_execution<'a>(
        &'a self,
        schema_registry: &SchemaRegistry,
        template_adapter_registry: &TransformTemplateAdapterRegistry,
        source: &FormatIdentity,
        target: &FormatIdentity,
    ) -> Result<DirectConversionExecution<'a>, ConversionExecutionError> {
        self.resolve_direct_execution_with_options(
            schema_registry,
            template_adapter_registry,
            source,
            target,
            ConversionLookupOptions::default(),
        )
    }

    pub fn resolve_direct_execution_with_options<'a>(
        &'a self,
        schema_registry: &SchemaRegistry,
        template_adapter_registry: &TransformTemplateAdapterRegistry,
        source: &FormatIdentity,
        target: &FormatIdentity,
        options: ConversionLookupOptions,
    ) -> Result<DirectConversionExecution<'a>, ConversionExecutionError> {
        let selection =
            self.select_direct_edge_with_options(schema_registry, source, target, options)?;
        let execution =
            resolve_descriptor_execution(selection.descriptor, template_adapter_registry)?;
        Ok(DirectConversionExecution {
            source: selection.source,
            target: selection.target,
            descriptor: selection.descriptor,
            execution,
        })
    }
}

fn resolve_descriptor_execution(
    descriptor: &ConversionDescriptor,
    template_adapter_registry: &TransformTemplateAdapterRegistry,
) -> Result<ConversionExecution, ConversionExecutionError> {
    match descriptor.implementation {
        ConversionImplementation::Rust => {
            let rust_symbol = descriptor.rust_symbol.clone().ok_or_else(|| {
                ConversionExecutionError::MissingRustSymbol {
                    converter_id: descriptor.id.clone(),
                }
            })?;
            Ok(ConversionExecution::Rust { rust_symbol })
        }
        ConversionImplementation::Cemt => {
            let template = descriptor.template.clone().ok_or_else(|| {
                ConversionExecutionError::MissingTemplate {
                    converter_id: descriptor.id.clone(),
                }
            })?;
            resolve_cemt_descriptor_execution(descriptor, template, template_adapter_registry)
        }
    }
}

fn resolve_cemt_descriptor_execution(
    descriptor: &ConversionDescriptor,
    template: ConversionTemplateDescriptor,
    template_adapter_registry: &TransformTemplateAdapterRegistry,
) -> Result<ConversionExecution, ConversionExecutionError> {
    let template_identity = FormatIdentity {
        content_type: Some(template.content_type.clone()),
        schema: template.schema.clone(),
        ..FormatIdentity::default()
    };
    let (adapter_id, capability) = match template_adapter_registry
        .select_adapter(&template_identity)
    {
        TransformTemplateAdapterLookup::Matched(adapter) => (adapter.id(), adapter.capability()),
        TransformTemplateAdapterLookup::Ambiguous(adapter_ids) => {
            return cemt_rust_fallback_or_error(
                descriptor,
                None,
                format!(
                    "template identity matched multiple adapters: {}",
                    adapter_ids.join(", ")
                ),
            );
        }
        TransformTemplateAdapterLookup::Unsupported => {
            return cemt_rust_fallback_or_error(
                descriptor,
                None,
                format!(
                    "no template adapter supports content type `{}`",
                    template.content_type
                ),
            );
        }
    };

    if descriptor.readiness == ConversionReadiness::Planned {
        return cemt_rust_fallback_or_error(
            descriptor,
            Some(adapter_id),
            "CEMT converter readiness is planned".to_owned(),
        );
    }

    if capability == TransformTemplateAdapterCapability::Executable {
        return Ok(ConversionExecution::CemtTemplate {
            adapter_id,
            template,
        });
    }

    cemt_rust_fallback_or_error(
        descriptor,
        Some(adapter_id),
        format!("template adapter `{adapter_id}` is selector-only"),
    )
}

fn cemt_rust_fallback_or_error(
    descriptor: &ConversionDescriptor,
    template_adapter_id: Option<&'static str>,
    reason: String,
) -> Result<ConversionExecution, ConversionExecutionError> {
    let Some(fallback) = descriptor.rust_fallback.as_ref() else {
        return Err(ConversionExecutionError::CemtExecutionUnavailable {
            converter_id: descriptor.id.clone(),
            reason,
        });
    };

    let configured_reason = fallback.reason.trim();
    let reason = if configured_reason.is_empty() {
        reason
    } else if reason.is_empty() || reason == configured_reason {
        configured_reason.to_owned()
    } else {
        format!("{configured_reason}; {reason}")
    };

    Ok(ConversionExecution::RustFallback {
        rust_symbol: fallback.rust_symbol.clone(),
        reason,
        template_adapter_id,
    })
}

fn descriptor_can_plan(
    descriptor: &ConversionDescriptor,
    options: ConversionLookupOptions,
) -> bool {
    if options.include_explicit_only {
        return true;
    }
    descriptor.implicit && !descriptor.explicit_only
}

fn descriptor_rank(descriptor: &ConversionDescriptor) -> (u32, u8) {
    let implementation_rank = match descriptor.implementation {
        ConversionImplementation::Cemt => 0,
        ConversionImplementation::Rust => 1,
    };
    (descriptor.cost, implementation_rank)
}

pub fn resolve_conversion_identity(
    identity: &FormatIdentity,
    schema_registry: &SchemaRegistry,
) -> Result<ResolvedConversionIdentity, ConversionIdentityError> {
    resolve_identity(identity, schema_registry)
}

fn resolve_identity(
    identity: &FormatIdentity,
    schema_registry: &SchemaRegistry,
) -> Result<ResolvedConversionIdentity, ConversionIdentityError> {
    let schema = identity
        .schema
        .as_deref()
        .map(str::trim)
        .filter(|schema| !schema.is_empty());

    if let Some(content_type) = identity
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|content_type| !content_type.is_empty())
    {
        return resolve_content_type_identity(content_type, schema, schema_registry);
    }

    if let Some(schema) = schema {
        let descriptor = schema_registry.schema(schema).ok_or_else(|| {
            ConversionIdentityError::UnknownSchema {
                schema: schema.to_owned(),
            }
        })?;
        return Ok(ResolvedConversionIdentity {
            content_type: primary_content_type_essence(descriptor)?,
            schema: descriptor.schema_uri.clone(),
        });
    }

    let namespaces = namespace_values(identity);
    if !namespaces.is_empty() {
        return resolve_namespace_identity(&namespaces, schema_registry);
    }

    Err(ConversionIdentityError::EmptyIdentity)
}

fn resolve_content_type_identity(
    content_type: &str,
    schema: Option<&str>,
    schema_registry: &SchemaRegistry,
) -> Result<ResolvedConversionIdentity, ConversionIdentityError> {
    let essence = content_type_essence(content_type);
    let descriptors = schema_registry.lookup_content_type(&essence);
    let candidate_schemas = descriptors
        .iter()
        .map(|descriptor| descriptor.schema_uri.clone())
        .collect::<Vec<_>>();

    if candidate_schemas.is_empty() {
        return Err(ConversionIdentityError::UnknownContentType {
            content_type: essence,
        });
    }

    let descriptor = if let Some(schema) = schema {
        descriptors
            .into_iter()
            .find(|descriptor| descriptor.schema_uri == schema)
            .ok_or_else(|| ConversionIdentityError::SchemaMismatch {
                content_type: essence.clone(),
                schema: schema.to_owned(),
                candidate_schemas,
            })?
    } else {
        match descriptors.as_slice() {
            [descriptor] => *descriptor,
            _ => {
                return Err(ConversionIdentityError::AmbiguousContentType {
                    content_type: essence,
                    schema_uris: candidate_schemas,
                });
            }
        }
    };

    Ok(ResolvedConversionIdentity {
        content_type: essence,
        schema: descriptor.schema_uri.clone(),
    })
}

fn resolve_namespace_identity(
    namespaces: &[String],
    schema_registry: &SchemaRegistry,
) -> Result<ResolvedConversionIdentity, ConversionIdentityError> {
    let mut descriptors = BTreeMap::<String, &SchemaDescriptor>::new();
    for namespace in namespaces {
        for descriptor in schema_registry.lookup_namespace(namespace) {
            descriptors.insert(descriptor.schema_uri.clone(), descriptor);
        }
    }

    match descriptors.len() {
        0 => Err(ConversionIdentityError::UnknownNamespace {
            namespaces: namespaces.to_vec(),
        }),
        1 => {
            let descriptor = descriptors.values().next().expect("descriptor exists");
            Ok(ResolvedConversionIdentity {
                content_type: primary_content_type_essence(descriptor)?,
                schema: descriptor.schema_uri.clone(),
            })
        }
        _ => Err(ConversionIdentityError::AmbiguousNamespace {
            namespaces: namespaces.to_vec(),
            schema_uris: descriptors.keys().cloned().collect(),
        }),
    }
}

fn namespace_values(identity: &FormatIdentity) -> Vec<String> {
    let mut namespaces = BTreeSet::new();
    if let Some(namespace) = identity.default_namespace.as_deref().map(str::trim) {
        if !namespace.is_empty() {
            namespaces.insert(namespace.to_owned());
        }
    }
    for namespace in identity
        .namespaces
        .values()
        .map(|namespace| namespace.trim())
    {
        if !namespace.is_empty() {
            namespaces.insert(namespace.to_owned());
        }
    }
    namespaces.into_iter().collect()
}

fn primary_content_type_essence(
    descriptor: &SchemaDescriptor,
) -> Result<String, ConversionIdentityError> {
    descriptor
        .content_types
        .iter()
        .find(|content_type| content_type.role == SchemaContentTypeRole::Primary)
        .map(|content_type| content_type.essence.clone())
        .ok_or_else(|| ConversionIdentityError::SchemaHasNoPrimaryContentType {
            schema: descriptor.schema_uri.clone(),
        })
}

pub fn builtin_conversion_descriptors() -> Vec<ConversionDescriptor> {
    vec![
        rust_edge(
            "cem-ml-to-dom-projection-rust",
            "cem-ml",
            endpoint(CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI),
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            "CemMlDomProjectionConverter",
            "lossless",
            100,
        ),
        rust_edge(
            "cem-ml-to-ast-projection-rust",
            "cem-ml",
            endpoint(CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI),
            endpoint(
                CEM_AST_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
            ),
            "CemMlAstProjectionConverter",
            "lossless",
            100,
        ),
        rust_edge(
            "cem-ml-to-events-projection-rust",
            "cem-ml",
            endpoint(CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI),
            endpoint(
                CEM_EVENTS_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            ),
            "CemMlEventsProjectionConverter",
            "lossless",
            100,
        ),
        rust_edge(
            "html-to-cem-dom-projection-rust",
            "html",
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            "Html5RecoveryConverter",
            "recovery",
            50,
        ),
        rust_edge(
            "xml-to-cem-dom-projection-rust",
            "xml",
            endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            "XmlDomProjectionConverter",
            "lossless",
            80,
        ),
        planned_cemt_edge_with_rust_fallback(
            "cem-dom-projection-to-html-cemt",
            "cem-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            "HtmlExportConverter",
            "CEMT DOM-to-HTML serialization edge is registered before execution is wired",
            "serialization",
            100,
        ),
        rust_edge(
            "cem-dom-projection-to-html-rust",
            "cem-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            "HtmlExportConverter",
            "serialization",
            100,
        ),
        planned_cemt_edge_with_rust_fallback(
            "cem-dom-projection-to-xml-cemt",
            "cem-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
            "schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt",
            "XmlExportConverter",
            "CEMT DOM-to-XML serialization edge is registered before execution is wired",
            "serialization",
            100,
        ),
        rust_edge(
            "cem-dom-projection-to-xml-rust",
            "cem-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
            "XmlExportConverter",
            "serialization",
            100,
        ),
        rust_edge(
            "cem-dom-projection-to-json-debug-rust",
            "cem-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            "DomJsonDebugProjectionConverter",
            "debug-view",
            150,
        ),
        rust_edge(
            "cem-ast-projection-to-json-debug-rust",
            "cem-ast-projection",
            endpoint(
                CEM_AST_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
            ),
            endpoint(
                CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
            ),
            "AstJsonDebugProjectionConverter",
            "debug-view",
            150,
        ),
        rust_edge(
            "cem-events-projection-to-json-debug-rust",
            "cem-events-projection",
            endpoint(
                CEM_EVENTS_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            ),
            endpoint(
                CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
            ),
            "EventsJsonDebugProjectionConverter",
            "debug-view",
            150,
        ),
    ]
}

fn endpoint(content_type: &str, schema: &str) -> ConversionEndpoint {
    ConversionEndpoint::with_schema(content_type, schema)
}

fn planned_cemt_edge_with_rust_fallback(
    id: &str,
    package_id: &str,
    from: ConversionEndpoint,
    to: ConversionEndpoint,
    template_path: &str,
    rust_symbol: &str,
    fallback_reason: &str,
    lossiness: &str,
    cost: u32,
) -> ConversionDescriptor {
    ConversionDescriptor {
        id: id.to_owned(),
        package_id: package_id.to_owned(),
        from,
        to,
        implementation: ConversionImplementation::Cemt,
        readiness: ConversionReadiness::Planned,
        template: Some(ConversionTemplateDescriptor {
            path: template_path.to_owned(),
            content_type: content_type_essence(CEM_TRANSFORM_CONTENT_TYPE),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
        }),
        rust_symbol: None,
        rust_fallback: Some(ConversionRustFallbackDescriptor {
            rust_symbol: rust_symbol.to_owned(),
            reason: fallback_reason.to_owned(),
        }),
        streamable: true,
        lossiness: Some(lossiness.to_owned()),
        implicit: true,
        explicit_only: false,
        cost,
    }
}

fn rust_edge(
    id: &str,
    package_id: &str,
    from: ConversionEndpoint,
    to: ConversionEndpoint,
    rust_symbol: &str,
    lossiness: &str,
    cost: u32,
) -> ConversionDescriptor {
    ConversionDescriptor {
        id: id.to_owned(),
        package_id: package_id.to_owned(),
        from,
        to,
        implementation: ConversionImplementation::Rust,
        readiness: ConversionReadiness::Ready,
        template: None,
        rust_symbol: Some(rust_symbol.to_owned()),
        rust_fallback: None,
        streamable: true,
        lossiness: Some(lossiness.to_owned()),
        implicit: true,
        explicit_only: false,
        cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::TransformTemplateKind;
    use crate::schema::registry::{
        NamespaceClaim, SchemaContentType, SchemaDescriptor, CEM_TRANSFORM_CONTENT_TYPE,
        CEM_TRANSFORM_SCHEMA_URI, JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI,
    };
    use crate::transform_template::{
        TransformTemplateAdapter, TransformTemplateAdapterCapability,
        TransformTemplateAdapterRegistry,
    };

    fn identity(content_type: &str) -> FormatIdentity {
        FormatIdentity {
            content_type: Some(content_type.to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn identity_with_schema(content_type: &str, schema: &str) -> FormatIdentity {
        FormatIdentity {
            content_type: Some(content_type.to_owned()),
            schema: Some(schema.to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn descriptor(schema_uri: &str, content_type: SchemaContentType) -> SchemaDescriptor {
        SchemaDescriptor {
            package_id: schema_uri.rsplit('/').next().unwrap_or(schema_uri).into(),
            schema_uri: schema_uri.into(),
            version: "1.0.0".into(),
            source: "schema/test.cem".into(),
            content_types: vec![content_type],
            namespaces: Vec::new(),
            uses: Vec::new(),
        }
    }

    #[test]
    fn builtin_registry_selects_direct_edge_from_content_type_identity() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let registry = ConversionRegistry::with_builtin_converters();

        let selection = registry
            .select_direct_edge(
                &schemas,
                &identity("text/html; charset=utf-8"),
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
            )
            .unwrap();

        assert_eq!(selection.descriptor.id, "html-to-cem-dom-projection-rust");
        assert_eq!(selection.source.content_type, HTML_CONTENT_TYPE);
        assert_eq!(selection.source.schema, HTML_SCHEMA_URI);
        assert_eq!(
            selection.target.content_type,
            CEM_DOM_PROJECTION_CONTENT_TYPE
        );
        assert_eq!(selection.target.schema, CEM_DOM_PROJECTION_SCHEMA_URI);
        assert_eq!(
            selection.descriptor.implementation,
            ConversionImplementation::Rust
        );
        assert_eq!(selection.descriptor.readiness, ConversionReadiness::Ready);
        assert!(selection.descriptor.rust_fallback.is_none());
    }

    #[test]
    fn builtin_registry_prefers_cemt_primary_edge_with_rust_fallback() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let registry = ConversionRegistry::with_builtin_converters();

        let selection = registry
            .select_direct_edge(
                &schemas,
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
                &identity(HTML_CONTENT_TYPE),
            )
            .unwrap();

        assert_eq!(selection.descriptor.id, "cem-dom-projection-to-html-cemt");
        assert_eq!(
            selection.descriptor.implementation,
            ConversionImplementation::Cemt
        );
        assert_eq!(selection.descriptor.readiness, ConversionReadiness::Planned);

        let template = selection.descriptor.template.as_ref().unwrap();
        assert_eq!(
            template.path,
            "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
        );
        assert_eq!(template.content_type, CEM_TRANSFORM_CONTENT_TYPE);
        assert_eq!(template.schema.as_deref(), Some(CEM_TRANSFORM_SCHEMA_URI));

        let fallback = selection.descriptor.rust_fallback.as_ref().unwrap();
        assert_eq!(fallback.rust_symbol, "HtmlExportConverter");
        assert!(fallback.reason.contains("before execution is wired"));

        let rust_edge = registry
            .converter("cem-dom-projection-to-html-rust")
            .expect("rust fallback edge remains registered");
        assert_eq!(rust_edge.implementation, ConversionImplementation::Rust);
        assert_eq!(rust_edge.readiness, ConversionReadiness::Ready);
    }

    #[test]
    fn builtin_execution_resolves_ready_rust_edge() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let registry = ConversionRegistry::with_builtin_converters();
        let template_adapters = TransformTemplateAdapterRegistry::with_builtin_adapters();

        let execution = registry
            .resolve_direct_execution(
                &schemas,
                &template_adapters,
                &identity(HTML_CONTENT_TYPE),
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
            )
            .unwrap();

        assert_eq!(execution.descriptor.id, "html-to-cem-dom-projection-rust");
        assert_eq!(
            execution.execution,
            ConversionExecution::Rust {
                rust_symbol: "Html5RecoveryConverter".to_owned()
            }
        );
    }

    #[test]
    fn builtin_execution_resolves_planned_cemt_to_registered_rust_fallback() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let registry = ConversionRegistry::with_builtin_converters();
        let template_adapters = TransformTemplateAdapterRegistry::with_builtin_adapters();

        let execution = registry
            .resolve_direct_execution(
                &schemas,
                &template_adapters,
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
                &identity(HTML_CONTENT_TYPE),
            )
            .unwrap();

        assert_eq!(execution.descriptor.id, "cem-dom-projection-to-html-cemt");
        assert_eq!(
            execution.source.content_type,
            CEM_DOM_PROJECTION_CONTENT_TYPE
        );
        assert_eq!(execution.target.content_type, HTML_CONTENT_TYPE);
        match &execution.execution {
            ConversionExecution::RustFallback {
                rust_symbol,
                reason,
                template_adapter_id,
            } => {
                assert_eq!(rust_symbol, "HtmlExportConverter");
                assert!(reason.contains("before execution is wired"));
                assert!(reason.contains("readiness is planned"));
                assert_eq!(*template_adapter_id, Some("cem-native-template"));
            }
            other => panic!("expected Rust fallback execution, got {other:?}"),
        }
    }

    #[derive(Clone)]
    struct ExecutableCemtAdapter;

    impl TransformTemplateAdapter for ExecutableCemtAdapter {
        fn id(&self) -> &'static str {
            "executable-cemt-test"
        }

        fn kind(&self) -> TransformTemplateKind {
            TransformTemplateKind::CemNative
        }

        fn capability(&self) -> TransformTemplateAdapterCapability {
            TransformTemplateAdapterCapability::Executable
        }

        fn matches_template(&self, identity: &FormatIdentity) -> bool {
            identity
                .content_type
                .as_deref()
                .is_some_and(|content_type| content_type == CEM_TRANSFORM_CONTENT_TYPE)
                || identity
                    .schema
                    .as_deref()
                    .is_some_and(|schema| schema == CEM_TRANSFORM_SCHEMA_URI)
        }
    }

    #[test]
    fn ready_cemt_execution_uses_executable_template_adapter() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let mut registry = ConversionRegistry::new();
        registry
            .register(ConversionDescriptor {
                id: "dom-to-html-cemt-ready".to_owned(),
                package_id: "cem-dom-projection".to_owned(),
                from: endpoint(
                    CEM_DOM_PROJECTION_CONTENT_TYPE,
                    CEM_DOM_PROJECTION_SCHEMA_URI,
                ),
                to: endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                implementation: ConversionImplementation::Cemt,
                readiness: ConversionReadiness::Ready,
                template: Some(ConversionTemplateDescriptor {
                    path: "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
                        .to_owned(),
                    content_type: CEM_TRANSFORM_CONTENT_TYPE.to_owned(),
                    schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                }),
                rust_symbol: None,
                rust_fallback: None,
                streamable: true,
                lossiness: Some("serialization".to_owned()),
                implicit: true,
                explicit_only: false,
                cost: 1,
            })
            .unwrap();
        let mut template_adapters = TransformTemplateAdapterRegistry::new();
        template_adapters.register(ExecutableCemtAdapter);

        let execution = registry
            .resolve_direct_execution(
                &schemas,
                &template_adapters,
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
                &identity(HTML_CONTENT_TYPE),
            )
            .unwrap();

        match execution.execution {
            ConversionExecution::CemtTemplate {
                adapter_id,
                template,
            } => {
                assert_eq!(adapter_id, "executable-cemt-test");
                assert_eq!(
                    template.path,
                    "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
                );
            }
            other => panic!("expected CEMT template execution, got {other:?}"),
        }
    }

    #[test]
    fn explicit_schema_must_match_content_type_owner() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let registry = ConversionRegistry::with_builtin_converters();

        let error = registry
            .select_direct_edge(
                &schemas,
                &identity_with_schema(HTML_CONTENT_TYPE, XML_SCHEMA_URI),
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ConversionLookupError::SourceIdentity(ConversionIdentityError::SchemaMismatch { .. })
        ));
    }

    #[test]
    fn content_type_ambiguity_can_be_resolved_by_explicit_schema() {
        let mut schemas = SchemaRegistry::new();
        schemas
            .register(descriptor(
                "https://cem.dev/ns/test-a/1",
                SchemaContentType::primary("application/vnd.shared+cem"),
            ))
            .unwrap();
        schemas
            .register(descriptor(
                "https://cem.dev/ns/test-b/1",
                SchemaContentType::alias("application/vnd.shared+cem"),
            ))
            .unwrap();
        schemas
            .register(descriptor(
                JSON_VALUE_SCHEMA_URI,
                SchemaContentType::primary(JSON_CONTENT_TYPE),
            ))
            .unwrap();

        let mut registry = ConversionRegistry::new();
        registry
            .register(rust_edge(
                "test-b-to-json-rust",
                "test-b",
                ConversionEndpoint::with_schema(
                    "application/vnd.shared+cem",
                    "https://cem.dev/ns/test-b/1",
                ),
                endpoint(JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI),
                "TestBToJsonConverter",
                "lossless",
                1,
            ))
            .unwrap();

        let ambiguous = registry
            .select_direct_edge(
                &schemas,
                &identity("application/vnd.shared+cem"),
                &identity(JSON_CONTENT_TYPE),
            )
            .unwrap_err();
        assert!(matches!(
            ambiguous,
            ConversionLookupError::SourceIdentity(
                ConversionIdentityError::AmbiguousContentType { .. }
            )
        ));

        let selection = registry
            .select_direct_edge(
                &schemas,
                &identity_with_schema("application/vnd.shared+cem", "https://cem.dev/ns/test-b/1"),
                &identity(JSON_CONTENT_TYPE),
            )
            .unwrap();
        assert_eq!(selection.descriptor.id, "test-b-to-json-rust");
    }

    #[test]
    fn namespace_identity_resolves_to_schema_primary_content_type() {
        let mut schemas = SchemaRegistry::new();
        schemas
            .register(SchemaDescriptor {
                package_id: "test".into(),
                schema_uri: "https://cem.dev/ns/test/1".into(),
                version: "1.0.0".into(),
                source: "schema/test.cem".into(),
                content_types: vec![SchemaContentType::primary("application/vnd.test+cem")],
                namespaces: vec![NamespaceClaim::new(Some("test"), "urn:test")],
                uses: Vec::new(),
            })
            .unwrap();

        let resolved = resolve_conversion_identity(
            &FormatIdentity {
                default_namespace: Some("urn:test".into()),
                ..FormatIdentity::default()
            },
            &schemas,
        )
        .unwrap();

        assert_eq!(resolved.content_type, "application/vnd.test+cem");
        assert_eq!(resolved.schema, "https://cem.dev/ns/test/1");
    }

    #[test]
    fn explicit_only_edges_are_excluded_from_implicit_lookup() {
        let mut schemas = SchemaRegistry::new();
        schemas
            .register(descriptor(
                "https://cem.dev/ns/source/1",
                SchemaContentType::primary("application/vnd.source+cem"),
            ))
            .unwrap();
        schemas
            .register(descriptor(
                "https://cem.dev/ns/target/1",
                SchemaContentType::primary("application/vnd.target+cem"),
            ))
            .unwrap();

        let mut edge = rust_edge(
            "source-to-target-explicit-rust",
            "source",
            ConversionEndpoint::with_schema(
                "application/vnd.source+cem",
                "https://cem.dev/ns/source/1",
            ),
            ConversionEndpoint::with_schema(
                "application/vnd.target+cem",
                "https://cem.dev/ns/target/1",
            ),
            "SourceToTargetConverter",
            "lossless",
            1,
        );
        edge.implicit = false;
        edge.explicit_only = true;

        let mut registry = ConversionRegistry::new();
        registry.register(edge).unwrap();

        let source = identity("application/vnd.source+cem");
        let target = identity("application/vnd.target+cem");
        let implicit = registry
            .select_direct_edge(&schemas, &source, &target)
            .unwrap_err();
        assert!(matches!(
            implicit,
            ConversionLookupError::NoDirectEdge { .. }
        ));

        let explicit = registry
            .select_direct_edge_with_options(
                &schemas,
                &source,
                &target,
                ConversionLookupOptions::explicit(),
            )
            .unwrap();
        assert_eq!(explicit.descriptor.id, "source-to-target-explicit-rust");
    }

    #[test]
    fn duplicate_converter_ids_are_rejected() {
        let mut registry = ConversionRegistry::new();
        let edge = rust_edge(
            "duplicate",
            "test",
            endpoint(CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI),
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            "TestConverter",
            "lossless",
            1,
        );
        registry.register(edge.clone()).unwrap();

        assert_eq!(
            registry.register(edge).unwrap_err(),
            ConversionRegistryError::DuplicateConverterId {
                id: "duplicate".into()
            }
        );
    }
}
