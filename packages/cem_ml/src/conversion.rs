//! Schema-owned content conversion registry.
//!
//! This module is the planning and dispatch-side contract for schema package
//! converter edges. Runtime execution still flows through the existing
//! lifecycle and transform-template adapters.

pub use crate::conversion_output::CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE;
use crate::conversion_output::{
    cemt_output_function_descriptor, default_formatter_tab_size, failed_pipeline_execution,
    output_pipeline_diagnostic, parse_formatter_line_ending_option,
    parse_positive_formatter_usize_option, resolve_formatter_line_ending,
    wrap_html_pre_container_artifact, CemtOutputFunctionDescriptorSpec, FormatterLineEndingMode,
};
use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{
    FormatIdentity, TemplateInput, TransformExecutionPolicy, TransformTemplateEntrypoint,
    TransformTemplateKind,
};
use crate::events::cem::CemEventNormalizer;
use crate::interpreter::OutputSpan;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::run_config::ScopeConfig;
use crate::schema::package_loader::{load_builtin_schema_package, BuiltinSchemaPackage};
use crate::schema::package_sources::{
    builtin_schema_package_artifact_source, builtin_schema_package_source,
};
use crate::schema::registry::{
    content_type_essence, SchemaContentTypeRole, SchemaDescriptor, SchemaRegistry,
    CEM_AST_PROJECTION_SCHEMA_URI, CEM_DOM_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
    CEM_EVENTS_PROJECTION_SCHEMA_URI, CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, CEM_QL_SCHEMA_URI,
    CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, CSV_CONTENT_TYPE, CSV_SCHEMA_URI,
    HTML_CONTENT_TYPE, HTML_SCHEMA_URI, JSON_CONTENT_TYPE, JSON_SCHEMA_CONTENT_TYPE,
    JSON_SCHEMA_SCHEMA_URI, JSON_VALUE_SCHEMA_URI, MARKDOWN_CONTENT_TYPE, MARKDOWN_SCHEMA_URI,
    XML_CONTENT_TYPE, XML_SCHEMA_URI, YAML_CONTENT_TYPE, YAML_SCHEMA_URI,
};
use crate::source::{BytesSource, SourceId};
use crate::source_map::SourceMapStack;
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::html::HtmlTokenizer;
use crate::tokenizer::xml::XmlTokenizer;
use crate::tokenizer::{SchemaTokenKind, SchemaTokenizer};
use crate::transform_template::{
    compose_transform_template_encoded_text_artifacts,
    evaluate_transform_template_encode_expressions, execute_transform_template_encode_binding,
    parse_cem_native_template_module_options, parse_transform_template_output_color_type,
    validate_transform_template_artifact_function_contract, TransformTemplateAdapter,
    TransformTemplateAdapterCapability, TransformTemplateAdapterError,
    TransformTemplateAdapterExecutionPhase, TransformTemplateAdapterLookup,
    TransformTemplateAdapterRegistry, TransformTemplateAdapterResult,
    TransformTemplateArtifactFunctionContract, TransformTemplateColorOutputProfile,
    TransformTemplateCompileRequest, TransformTemplateCompileResponse,
    TransformTemplateCompiledArtifact, TransformTemplateDataArtifact,
    TransformTemplateEncodeBinding, TransformTemplateEncodeBindingRequest,
    TransformTemplateEncodeEvaluationContext, TransformTemplateEncodeExpression,
    TransformTemplateEncodeImplementationRegistry, TransformTemplateEncodeOptions,
    TransformTemplateEncodedArtifact, TransformTemplateEncodedArtifactIdentity,
    TransformTemplateEncodedArtifactInsertionContext, TransformTemplateEncodedArtifactMode,
    TransformTemplateEncodingTarget, TransformTemplateEvaluatedEncodeExpression,
    TransformTemplateHtmlColorMode, TransformTemplateModuleOptions,
    TransformTemplateModuleParseRequest, TransformTemplateModulePreflight,
    TransformTemplateOutputArtifact, TransformTemplateOutputColorSelection,
    TransformTemplateOutputFunctionDescriptor, TransformTemplateOutputFunctionImplementation,
    TransformTemplateOutputFunctionKind, TransformTemplateOutputFunctionRegistry,
    TransformTemplateOutputProducedKind, TransformTemplateRenderRequest,
    TransformTemplateRenderResponse, TransformTemplateSourceMapPolicy,
    TransformTemplateTargetSyntaxKind, TransformTemplateTargetSyntaxRules,
    TransformTemplateTerminalColorCapability,
};
use crate::validation::csv::{generic_data_ast_to_csv_cemt_subject, CsvDocumentAst};
use crate::validation::generic_data::GenericDataDocumentAst;
use crate::validation::json::{generic_data_ast_to_json_cemt_subject, JsonDocumentAst};
use crate::validation::json_schema::JsonSchemaDocumentAst;
use crate::validation::markdown::MarkdownDocumentAst;
use crate::validation::yaml::{generic_data_ast_to_yaml_cemt_subject, YamlDocumentAst};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const CONVERSION_PARITY_NATIVE_PAIR_MISSING_CODE: &str =
    "cem.converter.parity_native_pair_missing";
pub const CONVERSION_PARITY_CEMT_PAIR_MISSING_CODE: &str = "cem.converter.parity_cemt_pair_missing";
pub const CONVERSION_PARITY_MODE_MISSING_CODE: &str = "cem.converter.parity_mode_missing";
pub const CONVERSION_PARITY_DRIFT_CODE: &str = "cem.converter.parity_drift";
pub const CONVERSION_PARITY_FIXTURE_EXECUTION_CODE: &str = "cem.converter.parity_fixture_execution";
pub const CONVERSION_PARITY_FIXTURE_LOAD_CODE: &str = "cem.converter.parity_fixture_load";
pub const CONVERSION_OUTPUT_SYNTAX_MISSING_CODE: &str = "cem.converter.output_syntax_missing";
pub const CONVERSION_OUTPUT_CATEGORY_MISSING_CODE: &str = "cem.converter.output_category_missing";
pub const CONVERSION_OUTPUT_UNSUPPORTED_CATEGORY_CODE: &str =
    "cem.converter.output_unsupported_category";
pub const CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE: &str = "cem.converter.output_context_mismatch";
pub const CONVERSION_OUTPUT_COLOR_PROFILE_UNSAFE_CODE: &str =
    "cem.converter.output_color_profile_unsafe";
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
    pub entrypoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionParityFixtureDescriptor {
    pub id: String,
    pub path: String,
    pub content_type: Option<String>,
    pub schema: Option<String>,
    pub expected_diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionRustFallbackDescriptor {
    pub rust_symbol: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionOutputSyntax {
    Html,
    Xml,
    Json,
    Yaml,
    Csv,
    Css,
    Markdown,
    Cemt,
    Text,
    Binary,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConversionPlanningDomain {
    ContentTypeConversion,
    SchemaOutputProduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionParityMode {
    ByteExact,
    TokenEquivalent,
    ParseEquivalent,
    DiagnosticEquivalent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversionOutputContractDescriptor {
    pub output_syntax: Option<ConversionOutputSyntax>,
    pub encoding_category: Option<String>,
    pub formatter_profile: Option<String>,
    pub color_profile: Option<String>,
    pub parity: Option<ConversionParityMode>,
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
    pub output_contract: ConversionOutputContractDescriptor,
    pub parity_fixtures: Vec<ConversionParityFixtureDescriptor>,
    pub implicit: bool,
    pub explicit_only: bool,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionPackageArtifactDescriptor {
    pub package_id: String,
    pub kind: String,
    pub path: String,
    pub content_type: Option<String>,
    pub schema: Option<String>,
    pub target_content_type: Option<String>,
    pub target_schema: Option<String>,
    pub target_category: Option<String>,
    pub function_name: Option<String>,
    pub function_profile: Option<String>,
    pub formatter_profile: Option<String>,
    pub color_profile: Option<String>,
    pub generated: bool,
}

impl ConversionDescriptor {
    pub fn planning_domain(&self) -> ConversionPlanningDomain {
        if descriptor_is_schema_output_producer(self) {
            ConversionPlanningDomain::SchemaOutputProduction
        } else {
            ConversionPlanningDomain::ContentTypeConversion
        }
    }
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
pub enum ConversionManifestError {
    MissingPackageElement,
    ArtifactContract {
        package_id: String,
        path: String,
        message: String,
    },
    ConverterTemplateContract {
        package_id: String,
        converter_id: String,
        path: String,
        message: String,
    },
}

impl std::fmt::Display for ConversionManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPackageElement => {
                write!(f, "schema package manifest has no package element")
            }
            Self::ArtifactContract {
                package_id,
                path,
                message,
            } => write!(
                f,
                "schema package `{package_id}` artifact `{path}` has invalid CEMT contract: {message}"
            ),
            Self::ConverterTemplateContract {
                package_id,
                converter_id,
                path,
                message,
            } => write!(
                f,
                "schema package `{package_id}` converter `{converter_id}` template `{path}` has invalid CEMT contract: {message}"
            ),
        }
    }
}

impl std::error::Error for ConversionManifestError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionParityFixtureLoadError {
    Read {
        converter_id: String,
        fixture_id: String,
        path: String,
        message: String,
    },
}

impl std::fmt::Display for ConversionParityFixtureLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read {
                converter_id,
                fixture_id,
                path,
                message,
            } => write!(
                f,
                "converter `{converter_id}` parity fixture `{fixture_id}` could not read `{path}`: {message}"
            ),
        }
    }
}

impl std::error::Error for ConversionParityFixtureLoadError {}

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
    pub planning_domain: Option<ConversionPlanningDomain>,
}

impl ConversionLookupOptions {
    pub fn implicit() -> Self {
        Self {
            include_explicit_only: false,
            planning_domain: None,
        }
    }

    pub fn explicit() -> Self {
        Self {
            include_explicit_only: true,
            planning_domain: None,
        }
    }

    pub fn content_type_conversion() -> Self {
        Self::implicit().with_planning_domain(ConversionPlanningDomain::ContentTypeConversion)
    }

    pub fn schema_output_production() -> Self {
        Self::implicit().with_planning_domain(ConversionPlanningDomain::SchemaOutputProduction)
    }

    pub fn with_planning_domain(mut self, planning_domain: ConversionPlanningDomain) -> Self {
        self.planning_domain = Some(planning_domain);
        self
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

#[derive(Debug, Clone)]
pub struct ConversionParityContract<'a> {
    pub cemt: &'a ConversionDescriptor,
    pub native: &'a ConversionDescriptor,
    pub mode: ConversionParityMode,
}

#[derive(Debug, Clone)]
pub struct ConversionParityFixture {
    pub id: String,
    pub input: Value,
    pub expected_diagnostics: Vec<Diagnostic>,
    pub expected_diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ConversionParityFixtureExecution {
    pub output: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
}

pub trait ConversionParityFixtureExecutor {
    fn execute_conversion_parity_fixture(
        &self,
        descriptor: &ConversionDescriptor,
        fixture: &ConversionParityFixture,
    ) -> ConversionParityFixtureExecution;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RustDomProjectionParityFixtureExecutor;

impl ConversionParityFixtureExecutor for RustDomProjectionParityFixtureExecutor {
    fn execute_conversion_parity_fixture(
        &self,
        descriptor: &ConversionDescriptor,
        fixture: &ConversionParityFixture,
    ) -> ConversionParityFixtureExecution {
        execute_rust_dom_projection_parity_fixture(descriptor, fixture)
    }
}

#[derive(Debug)]
pub struct CemtTemplateParityFixtureExecutor<'a> {
    package_root: PathBuf,
    template_adapter_registry: &'a TransformTemplateAdapterRegistry,
}

impl<'a> CemtTemplateParityFixtureExecutor<'a> {
    pub fn new(
        package_root: impl AsRef<Path>,
        template_adapter_registry: &'a TransformTemplateAdapterRegistry,
    ) -> Self {
        Self {
            package_root: package_root.as_ref().to_path_buf(),
            template_adapter_registry,
        }
    }
}

impl ConversionParityFixtureExecutor for CemtTemplateParityFixtureExecutor<'_> {
    fn execute_conversion_parity_fixture(
        &self,
        descriptor: &ConversionDescriptor,
        fixture: &ConversionParityFixture,
    ) -> ConversionParityFixtureExecution {
        match descriptor.implementation {
            ConversionImplementation::Cemt => execute_cemt_template_parity_fixture(
                descriptor,
                fixture,
                &self.package_root,
                self.template_adapter_registry,
            ),
            ConversionImplementation::Rust => {
                execute_rust_dom_projection_parity_fixture(descriptor, fixture)
            }
        }
    }
}

/// Executable bounded adapter for the packaged DOM-projection CEMT serializers.
///
/// It recognizes the current packaged DOM serializer templates for normal
/// conversion and parity verification; it is not a general-purpose CEMT
/// interpreter.
#[derive(Clone, Debug, Default)]
pub struct DomProjectionParityCemtAdapter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConversionDomProjectionParityOutput {
    Html,
    Xml,
}

impl TransformTemplateAdapter for DomProjectionParityCemtAdapter {
    fn id(&self) -> &'static str {
        "dom-projection-parity-cemt"
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
            .is_some_and(|content_type| {
                content_type_essence(content_type) == CEM_TRANSFORM_CONTENT_TYPE
            })
            || identity
                .schema
                .as_deref()
                .is_some_and(|schema| schema == CEM_TRANSFORM_SCHEMA_URI)
    }

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        let template = std::str::from_utf8(&request.template.bytes).map_err(|error| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                error.to_string(),
            )
        })?;
        if !template.contains(r#"template @name="emit-node""#)
            || !template.contains(r#"cem:for-each @select="$input.children""#)
        {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                "template is not a supported DOM projection converter",
            ));
        }
        let output = if template.contains(r#"node.kind == "cdata""#)
            || template.contains(r#"node.kind = "cdata""#)
        {
            "xml"
        } else if template.contains(r#"node.kind == "raw-text""#)
            || template.contains(r#"node.kind = "raw-text""#)
        {
            "html"
        } else {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                "DOM projection converter output kind is not recognized",
            ));
        };

        Ok(TransformTemplateCompileResponse {
            artifact: TransformTemplateCompiledArtifact::new(
                self.id(),
                self.kind(),
                request.template.uri.clone(),
                request.template.identity.clone(),
                request.entrypoint.clone(),
                serde_json::json!({ "domProjectionParityOutput": output }),
            ),
            diagnostics: Vec::new(),
        })
    }

    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        if conversion_dom_projection_parity_target_is_cem_tree(&request) {
            let tree = conversion_dom_projection_parity_cem_tree_document(
                &request.primary_input.value,
                request.target_scope,
            )
            .map_err(|message| {
                TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Render,
                    message,
                )
            })?;
            return Ok(TransformTemplateRenderResponse {
                output: TransformTemplateOutputArtifact {
                    uri: None,
                    identity: request.target.cloned(),
                    value: tree,
                    source_map: None,
                    output_spans: Vec::new(),
                },
                diagnostics: Vec::new(),
            });
        }

        let output = conversion_dom_projection_parity_output(&request).map_err(|message| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Render,
                message,
            )
        })?;
        let rendered =
            conversion_render_dom_projection_parity_document(&request.primary_input.value, output)
                .map_err(|message| {
                    TransformTemplateAdapterError::failed(
                        self.id(),
                        TransformTemplateAdapterExecutionPhase::Render,
                        message,
                    )
                })?;

        Ok(TransformTemplateRenderResponse {
            output: TransformTemplateOutputArtifact {
                uri: None,
                identity: request.target.cloned(),
                value: Value::String(rendered),
                source_map: None,
                output_spans: Vec::new(),
            },
            diagnostics: Vec::new(),
        })
    }
}

fn conversion_dom_projection_parity_target_is_cem_tree(
    request: &TransformTemplateRenderRequest<'_>,
) -> bool {
    request.target.is_some_and(|target| {
        target
            .content_type
            .as_deref()
            .is_some_and(|content_type| content_type_essence(content_type) == CEM_ML_CONTENT_TYPE)
            || target
                .schema
                .as_deref()
                .is_some_and(|schema| schema == CEM_ML_SCHEMA_URI)
    })
}

fn conversion_dom_projection_parity_output(
    request: &TransformTemplateRenderRequest<'_>,
) -> Result<ConversionDomProjectionParityOutput, String> {
    if let Some(content_type) = request
        .target
        .and_then(|target| target.content_type.as_deref())
    {
        let essence = content_type_essence(content_type);
        if essence == HTML_CONTENT_TYPE {
            return Ok(ConversionDomProjectionParityOutput::Html);
        }
        if essence == XML_CONTENT_TYPE {
            return Ok(ConversionDomProjectionParityOutput::Xml);
        }
    }

    match request
        .compiled
        .opaque
        .get("domProjectionParityOutput")
        .and_then(Value::as_str)
    {
        Some("html") => Ok(ConversionDomProjectionParityOutput::Html),
        Some("xml") => Ok(ConversionDomProjectionParityOutput::Xml),
        _ => Err("DOM projection converter target output kind is not recognized".to_owned()),
    }
}

fn conversion_render_dom_projection_parity_document(
    input: &Value,
    output: ConversionDomProjectionParityOutput,
) -> Result<String, String> {
    let children = conversion_template_input_children(input)?;
    let mut rendered = String::new();
    for child in children {
        conversion_render_dom_projection_parity_node(child, output, &mut rendered)?;
    }
    Ok(rendered)
}

fn conversion_render_dom_projection_parity_node(
    node: &Value,
    output: ConversionDomProjectionParityOutput,
    rendered: &mut String,
) -> Result<(), String> {
    match node.get("kind").and_then(Value::as_str).unwrap_or_default() {
        "element" => conversion_render_dom_projection_parity_element(node, output, rendered),
        "text" => {
            conversion_escape_text_into(rendered, conversion_dom_projection_parity_data(node));
            Ok(())
        }
        "whitespace" => {
            rendered.push_str(conversion_dom_projection_parity_data(node));
            Ok(())
        }
        "comment" => {
            rendered.push_str("<!--");
            rendered.push_str(conversion_dom_projection_parity_data(node));
            rendered.push_str("-->");
            Ok(())
        }
        "cdata" if output == ConversionDomProjectionParityOutput::Xml => {
            rendered.push_str("<![CDATA[");
            rendered.push_str(conversion_dom_projection_parity_data(node));
            rendered.push_str("]]>");
            Ok(())
        }
        "processing-instruction" if output == ConversionDomProjectionParityOutput::Xml => {
            let target = node
                .get("name")
                .or_else(|| node.get("target"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            rendered.push_str("<?");
            rendered.push_str(target);
            let data = conversion_dom_projection_parity_data(node);
            if !data.is_empty() {
                rendered.push(' ');
                rendered.push_str(data);
            }
            rendered.push_str("?>");
            Ok(())
        }
        "raw-text" if output == ConversionDomProjectionParityOutput::Html => {
            rendered.push_str(conversion_dom_projection_parity_data(node));
            Ok(())
        }
        _ => Ok(()),
    }
}

fn conversion_render_dom_projection_parity_element(
    node: &Value,
    output: ConversionDomProjectionParityOutput,
    rendered: &mut String,
) -> Result<(), String> {
    let name = conversion_dom_projection_parity_name(node)?;
    if name.local.starts_with('@') {
        return Ok(());
    }
    rendered.push('<');
    conversion_push_dom_projection_parity_name(rendered, &name);
    if let Some(attributes) = node.get("attributes").and_then(Value::as_array) {
        for attribute in attributes {
            conversion_render_dom_projection_parity_attribute(attribute, output, rendered)?;
        }
    }
    rendered.push('>');
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            conversion_render_dom_projection_parity_node(child, output, rendered)?;
        }
    }
    rendered.push_str("</");
    conversion_push_dom_projection_parity_name(rendered, &name);
    rendered.push('>');
    Ok(())
}

fn conversion_render_dom_projection_parity_attribute(
    attribute: &Value,
    output: ConversionDomProjectionParityOutput,
    rendered: &mut String,
) -> Result<(), String> {
    let name = conversion_dom_projection_parity_name(attribute)?;
    rendered.push(' ');
    conversion_push_dom_projection_parity_name(rendered, &name);
    match (output, attribute.get("value")) {
        (ConversionDomProjectionParityOutput::Html, Some(Value::Null) | None) => Ok(()),
        (_, value) => {
            rendered.push_str("=\"");
            if let Some(value) = value.and_then(Value::as_str) {
                match output {
                    ConversionDomProjectionParityOutput::Html => {
                        conversion_escape_html_attribute_into(rendered, value);
                    }
                    ConversionDomProjectionParityOutput::Xml => {
                        conversion_escape_xml_attribute_into(rendered, value);
                    }
                }
            }
            rendered.push('"');
            Ok(())
        }
    }
}

#[derive(Debug)]
struct ConversionDomProjectionParityName<'a> {
    namespace: &'a str,
    local: &'a str,
}

fn conversion_dom_projection_parity_name(
    node: &Value,
) -> Result<ConversionDomProjectionParityName<'_>, String> {
    let local = node
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "DOM projection node is missing a name".to_owned())?;
    let namespace = node
        .get("namespace")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(ConversionDomProjectionParityName { namespace, local })
}

fn conversion_push_dom_projection_parity_name(
    rendered: &mut String,
    name: &ConversionDomProjectionParityName<'_>,
) {
    if !name.namespace.is_empty() {
        rendered.push_str(name.namespace);
        rendered.push(':');
    }
    rendered.push_str(name.local);
}

fn conversion_dom_projection_parity_data(node: &Value) -> &str {
    node.get("data")
        .or_else(|| node.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn conversion_dom_projection_parity_cem_tree_document(
    input: &Value,
    target_scope: &ScopeConfig,
) -> Result<Value, String> {
    let children = conversion_template_input_children(input)?;
    let mut nodes = Vec::new();
    for child in children {
        if let Some(node) = conversion_dom_projection_parity_cem_tree_node(child)? {
            nodes.push(node);
        }
    }
    let formatter_profile = conversion_dom_projection_parity_formatter_profile(target_scope);
    Ok(serde_json::json!({
        "kind": "cem-tree",
        "contentType": CEM_ML_CONTENT_TYPE,
        "schema": CEM_ML_SCHEMA_URI,
        "category": "cem-tree",
        "mode": TransformTemplateEncodedArtifactMode::Document.as_str(),
        "canonical": true,
        "formatterProfile": formatter_profile,
        "formatNodes": [
            {
                "kind": "format-marker",
                "name": "cem.format-tree",
                "formatterRole": "formatter.boundary",
                "formatterProfile": formatter_profile,
            },
            {
                "kind": "format-decision",
                "name": "converter-cemt",
                "formatterRole": "formatter.converter",
                "formatterProfile": formatter_profile,
                "value": "converter CEMT produced formatted tree",
            }
        ],
        "nodes": nodes,
    }))
}

fn conversion_template_input_children(input: &Value) -> Result<&Vec<Value>, String> {
    if let Some(nodes) = input.as_array() {
        return Ok(nodes);
    }
    input
        .get("children")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "converter template input must be a CEM AST stream array or contain a children array"
                .to_owned()
        })
}

fn conversion_dom_projection_parity_formatter_profile(target_scope: &ScopeConfig) -> String {
    target_scope
        .cemt_formatter_profile
        .as_deref()
        .or(target_scope.cemt_formatter.as_deref())
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(|profile| match profile {
            "compact" | "pretty" | "tabular" => profile,
            "format-tree" | "cem.format-tree" | "canonical" => "compact",
            _ => profile,
        })
        .unwrap_or("compact")
        .to_owned()
}

fn conversion_dom_projection_parity_cem_tree_node(node: &Value) -> Result<Option<Value>, String> {
    let kind = node.get("kind").and_then(Value::as_str).unwrap_or_default();
    match kind {
        "text" | "whitespace" | "comment" | "cdata" | "raw-text" => Ok(Some(serde_json::json!({
            "kind": kind,
            "value": conversion_dom_projection_parity_data(node),
            "sourceMap": conversion_dom_projection_parity_source_map(node),
        }))),
        "processing-instruction" => {
            let name = conversion_dom_projection_parity_name(node)?;
            let mut fields = serde_json::Map::new();
            fields.insert(
                "kind".to_owned(),
                Value::String("processing-instruction".to_owned()),
            );
            fields.insert("target".to_owned(), Value::String(name.local.to_owned()));
            fields.insert(
                "value".to_owned(),
                Value::String(conversion_dom_projection_parity_data(node).to_owned()),
            );
            fields.insert(
                "sourceMap".to_owned(),
                conversion_dom_projection_parity_source_map(node),
            );
            Ok(Some(Value::Object(fields)))
        }
        _ => conversion_dom_projection_parity_cem_tree_element(node),
    }
}

fn conversion_dom_projection_parity_cem_tree_element(
    node: &Value,
) -> Result<Option<Value>, String> {
    let name = conversion_dom_projection_parity_name(node)?;
    if name.local.starts_with('@') {
        return Ok(None);
    }

    let mut fields = serde_json::Map::new();
    fields.insert("kind".to_owned(), Value::String("element".to_owned()));
    fields.insert("name".to_owned(), Value::String(name.local.to_owned()));
    fields.insert(
        "sourceMap".to_owned(),
        conversion_dom_projection_parity_source_map(node),
    );
    if !name.namespace.is_empty() {
        fields.insert(
            "namespace".to_owned(),
            Value::String(name.namespace.to_owned()),
        );
    }

    let attributes = node
        .get("attributes")
        .and_then(Value::as_array)
        .map(|attributes| {
            attributes
                .iter()
                .map(conversion_dom_projection_parity_cem_tree_attribute)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    fields.insert("attributes".to_owned(), Value::Array(attributes));

    let mut child_nodes = Vec::new();
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            if let Some(child_node) = conversion_dom_projection_parity_cem_tree_node(child)? {
                child_nodes.push(child_node);
            }
        }
    }
    fields.insert("children".to_owned(), Value::Array(child_nodes));
    fields.insert(
        "formatLayout".to_owned(),
        conversion_dom_projection_parity_format_layout(name.local),
    );

    Ok(Some(Value::Object(fields)))
}

fn conversion_dom_projection_parity_cem_tree_attribute(attribute: &Value) -> Result<Value, String> {
    let name = conversion_dom_projection_parity_name(attribute)?;
    let mut fields = serde_json::Map::new();
    fields.insert("kind".to_owned(), Value::String("attribute".to_owned()));
    fields.insert("name".to_owned(), Value::String(name.local.to_owned()));
    if !name.namespace.is_empty() {
        fields.insert(
            "namespace".to_owned(),
            Value::String(name.namespace.to_owned()),
        );
    }
    fields.insert(
        "value".to_owned(),
        attribute.get("value").cloned().unwrap_or(Value::Null),
    );
    fields.insert(
        "sourceMap".to_owned(),
        conversion_dom_projection_parity_source_map(attribute),
    );
    Ok(Value::Object(fields))
}

fn conversion_dom_projection_parity_format_layout(local_name: &str) -> Value {
    if local_name == "strong" {
        serde_json::json!({
            "kind": "format-decision",
            "formatterRole": "formatter.inline-emphasis",
            "value": "inline-emphasis",
        })
    } else {
        serde_json::json!({
            "kind": "format-decision",
            "formatterRole": "formatter.layout",
            "value": "inline",
        })
    }
}

fn conversion_dom_projection_parity_source_map(node: &Value) -> Value {
    node.get("sourceMap").cloned().unwrap_or(Value::Null)
}

#[derive(Debug, Clone)]
pub struct ConversionOutputSafetyContract<'a> {
    pub descriptor: &'a ConversionDescriptor,
    pub target: TransformTemplateEncodingTarget,
    pub options: TransformTemplateEncodeOptions,
    pub syntax_rules: TransformTemplateTargetSyntaxRules,
    pub insertion_context: TransformTemplateEncodedArtifactInsertionContext,
    pub produces: TransformTemplateOutputProducedKind,
    pub color_output_profile: Option<TransformTemplateColorOutputProfile>,
    pub pipeline: ConversionOutputPipeline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionOutputPipeline {
    pub stages: Vec<ConversionOutputPipelineStage>,
    pub cemt_target: TransformTemplateEncodingTarget,
    pub cemt_options: TransformTemplateEncodeOptions,
    pub cemt_insertion_context: TransformTemplateEncodedArtifactInsertionContext,
    pub cemt_produces: TransformTemplateOutputProducedKind,
    pub writer_insertion_context: TransformTemplateEncodedArtifactInsertionContext,
    pub writer_produces: TransformTemplateOutputProducedKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionPackageArtifactRead {
    pub uri: String,
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

pub type ConversionPackageArtifactReader<'a> = dyn Fn(&ConversionPackageArtifactDescriptor) -> Result<ConversionPackageArtifactRead, String>
    + 'a;

#[derive(Debug, Default)]
pub struct ConversionOutputPipelineArtifactCache {
    module_options: RefCell<BTreeMap<String, Result<TransformTemplateModuleOptions, String>>>,
}

#[derive(Clone, Copy)]
pub struct ConversionOutputPipelineEnvironment<'a> {
    pub schema_registry: &'a SchemaRegistry,
    pub conversion_registry: &'a ConversionRegistry,
    pub package_artifact_reader: Option<&'a ConversionPackageArtifactReader<'a>>,
    pub artifact_cache: Option<&'a ConversionOutputPipelineArtifactCache>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionOutputPipelineStage {
    Transform,
    Format,
    Color,
    Writer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionOutputPipelineStageExecution {
    CemtAdapter {
        adapter_id: String,
        function_name: String,
        body_function_name: Option<String>,
        fallback_function_name: Option<String>,
    },
    CemtFallback {
        function_name: String,
    },
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
    package_artifacts: Vec<ConversionPackageArtifactDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConversionPackageArtifactSelectionError {
    package_id: String,
    kind: String,
    requested_function_name: Option<String>,
    profile: Option<String>,
    candidates: Vec<String>,
}

impl fmt::Display for ConversionPackageArtifactSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let selector = if let Some(function_name) = self.requested_function_name.as_deref() {
            format!("function `{function_name}`")
        } else if let Some(profile) = self.profile.as_deref() {
            format!("profile `{profile}`")
        } else {
            "default profile".to_owned()
        };
        let selector_hint = match self.kind.as_str() {
            "formatter" => "set `cemtFormatter` or `--cemt-formatter`",
            "colorizer" => "set `cemtColorizer` or `--cemt-colorizer`",
            _ => "set an explicit CEMT output function selector",
        };
        write!(
            f,
            "schema package `{}` declares multiple `{}` CEMT artifacts for {}; {selector_hint} to select one explicitly: {}",
            self.package_id,
            self.kind,
            selector,
            self.candidates.join(", ")
        )
    }
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
        for artifact in builtin_conversion_package_artifacts() {
            registry.register_package_artifact(artifact);
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

    pub fn register_package_artifact(&mut self, artifact: ConversionPackageArtifactDescriptor) {
        self.package_artifacts.push(artifact);
    }

    pub fn package_artifacts(&self) -> impl Iterator<Item = &ConversionPackageArtifactDescriptor> {
        self.package_artifacts.iter()
    }

    pub fn select_package_artifact(
        &self,
        package_id: &str,
        kind: &str,
        content_type: Option<&str>,
        schema: Option<&str>,
    ) -> Option<&ConversionPackageArtifactDescriptor> {
        let content_type = content_type.map(content_type_essence);
        self.package_artifacts.iter().rev().find(|artifact| {
            artifact.package_id == package_id
                && artifact.kind == kind
                && content_type
                    .as_deref()
                    .is_none_or(|expected| artifact.content_type.as_deref() == Some(expected))
                && schema
                    .map(str::trim)
                    .filter(|expected| !expected.is_empty())
                    .is_none_or(|expected| artifact.schema.as_deref() == Some(expected))
        })
    }

    fn select_package_artifact_for_output_stage(
        &self,
        package_id: &str,
        kind: &str,
        content_type: Option<&str>,
        schema: Option<&str>,
        target: &TransformTemplateEncodingTarget,
        requested_function_name: Option<&str>,
        canonical_function_name: &str,
        formatter_profile: Option<&str>,
        color_profile: Option<&str>,
    ) -> Result<Option<&ConversionPackageArtifactDescriptor>, ConversionPackageArtifactSelectionError>
    {
        let content_type = content_type.map(content_type_essence);
        let target_content_type = content_type_essence(&target.content_type);
        let requested_function_name = requested_function_name
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let formatter_profile = formatter_profile
            .map(str::trim)
            .filter(|profile| !profile.is_empty());
        let color_profile = color_profile
            .map(str::trim)
            .filter(|profile| !profile.is_empty());
        let matches_stage_identity = |artifact: &ConversionPackageArtifactDescriptor| {
            artifact.package_id == package_id
                && artifact.kind == kind
                && content_type
                    .as_deref()
                    .is_none_or(|expected| artifact.content_type.as_deref() == Some(expected))
                && schema
                    .map(str::trim)
                    .filter(|expected| !expected.is_empty())
                    .is_none_or(|expected| artifact.schema.as_deref() == Some(expected))
                && artifact.target_content_type.as_deref() == Some(target_content_type.as_str())
                && artifact.target_schema.as_deref() == Some(target.schema.as_str())
                && artifact.target_category.as_deref() == Some(target.category.as_str())
                && formatter_profile
                    .is_none_or(|expected| artifact.formatter_profile.as_deref() == Some(expected))
                && color_profile
                    .is_none_or(|expected| artifact.color_profile.as_deref() == Some(expected))
        };
        let stage_matches = self
            .package_artifacts
            .iter()
            .filter(|artifact| matches_stage_identity(artifact))
            .collect::<Vec<_>>();

        if let Some(requested_function_name) = requested_function_name {
            return self.one_package_artifact_or_ambiguous(
                stage_matches
                    .into_iter()
                    .filter(|artifact| {
                        artifact.function_name.as_deref() == Some(requested_function_name)
                    })
                    .collect(),
                package_id,
                kind,
                Some(requested_function_name),
                formatter_profile.or(color_profile),
            );
        }

        let canonical_matches = stage_matches
            .iter()
            .copied()
            .filter(|artifact| artifact.function_name.as_deref() == Some(canonical_function_name))
            .collect::<Vec<_>>();
        match self.one_package_artifact_or_ambiguous(
            canonical_matches,
            package_id,
            kind,
            None,
            formatter_profile.or(color_profile),
        )? {
            Some(artifact) => return Ok(Some(artifact)),
            None => {}
        }

        self.one_package_artifact_or_ambiguous(
            stage_matches
                .into_iter()
                .filter(|artifact| {
                    package_artifact_function_matches_output_stage(
                        artifact,
                        canonical_function_name,
                        formatter_profile,
                        color_profile,
                    )
                })
                .collect(),
            package_id,
            kind,
            None,
            formatter_profile.or(color_profile),
        )
    }

    fn one_package_artifact_or_ambiguous<'a>(
        &self,
        candidates: Vec<&'a ConversionPackageArtifactDescriptor>,
        package_id: &str,
        kind: &str,
        requested_function_name: Option<&str>,
        profile: Option<&str>,
    ) -> Result<
        Option<&'a ConversionPackageArtifactDescriptor>,
        ConversionPackageArtifactSelectionError,
    > {
        match candidates.as_slice() {
            [] => Ok(None),
            [artifact] => Ok(Some(*artifact)),
            _ => {
                let function_names = candidates
                    .iter()
                    .filter_map(|artifact| artifact.function_name.as_deref())
                    .collect::<BTreeSet<_>>();
                if function_names.len() <= 1 {
                    return Ok(candidates.last().copied());
                }
                Err(ConversionPackageArtifactSelectionError {
                    package_id: package_id.to_owned(),
                    kind: kind.to_owned(),
                    requested_function_name: requested_function_name.map(str::to_owned),
                    profile: profile.map(str::to_owned),
                    candidates: candidates
                        .iter()
                        .map(|artifact| package_artifact_selection_label(artifact))
                        .collect(),
                })
            }
        }
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

    pub fn select_content_type_conversion_edge<'a>(
        &'a self,
        schema_registry: &SchemaRegistry,
        source: &FormatIdentity,
        target: &FormatIdentity,
    ) -> Result<DirectConversionSelection<'a>, ConversionLookupError> {
        self.select_direct_edge_with_options(
            schema_registry,
            source,
            target,
            ConversionLookupOptions::content_type_conversion(),
        )
    }

    pub fn select_schema_output_producer<'a>(
        &'a self,
        schema_registry: &SchemaRegistry,
        source: &FormatIdentity,
        target: &FormatIdentity,
    ) -> Result<DirectConversionSelection<'a>, ConversionLookupError> {
        self.select_direct_edge_with_options(
            schema_registry,
            source,
            target,
            ConversionLookupOptions::schema_output_production(),
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

    pub fn resolve_content_type_conversion_execution<'a>(
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
            ConversionLookupOptions::content_type_conversion(),
        )
    }

    pub fn resolve_schema_output_execution<'a>(
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
            ConversionLookupOptions::schema_output_production(),
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

    pub fn cemt_native_parity_contracts(
        &self,
    ) -> (Vec<ConversionParityContract<'_>>, Vec<Diagnostic>) {
        let mut contracts = Vec::new();
        let mut diagnostics = Vec::new();

        for cemt in self
            .descriptors_by_id
            .values()
            .filter(|descriptor| descriptor.implementation == ConversionImplementation::Cemt)
            .filter(|descriptor| descriptor.rust_fallback.is_some())
        {
            let Some(mode) = cemt.output_contract.parity else {
                diagnostics.push(conversion_parity_diagnostic(
                    CONVERSION_PARITY_MODE_MISSING_CODE,
                    format!(
                        "CEMT converter `{}` declares Rust fallback `{}` but no parity mode",
                        cemt.id,
                        cemt.rust_fallback
                            .as_ref()
                            .map(|fallback| fallback.rust_symbol.as_str())
                            .unwrap_or("<missing>")
                    ),
                ));
                continue;
            };
            let fallback = cemt.rust_fallback.as_ref().expect("filtered CEMT fallback");
            let native = self.descriptors_by_id.values().find(|descriptor| {
                descriptor.implementation == ConversionImplementation::Rust
                    && descriptor.rust_symbol.as_deref() == Some(fallback.rust_symbol.as_str())
                    && descriptor.from == cemt.from
                    && descriptor.to == cemt.to
            });
            let Some(native) = native else {
                diagnostics.push(conversion_parity_diagnostic(
                    CONVERSION_PARITY_NATIVE_PAIR_MISSING_CODE,
                    format!(
                        "CEMT converter `{}` declares Rust fallback `{}` but no matching Rust converter has the same source and target identity",
                        cemt.id, fallback.rust_symbol
                    ),
                ));
                continue;
            };

            contracts.push(ConversionParityContract { cemt, native, mode });
        }

        for native in self
            .descriptors_by_id
            .values()
            .filter(|descriptor| descriptor.implementation == ConversionImplementation::Rust)
            .filter(|descriptor| descriptor.readiness == ConversionReadiness::Ready)
            .filter(|descriptor| {
                descriptor.planning_domain() == ConversionPlanningDomain::SchemaOutputProduction
            })
        {
            let Some(native_symbol) = native.rust_symbol.as_deref() else {
                continue;
            };
            let has_cemt_pair = self.descriptors_by_id.values().any(|descriptor| {
                descriptor.implementation == ConversionImplementation::Cemt
                    && descriptor.from == native.from
                    && descriptor.to == native.to
                    && descriptor
                        .rust_fallback
                        .as_ref()
                        .map(|fallback| fallback.rust_symbol.as_str())
                        == Some(native_symbol)
            });
            if !has_cemt_pair {
                diagnostics.push(conversion_parity_diagnostic(
                    CONVERSION_PARITY_CEMT_PAIR_MISSING_CODE,
                    format!(
                        "Rust converter `{}` has no matching CEMT converter with the same source and target identity and fallback symbol `{native_symbol}`",
                        native.id
                    ),
                ));
            }
        }

        (contracts, diagnostics)
    }

    pub fn cemt_output_safety_contracts(
        &self,
    ) -> (Vec<ConversionOutputSafetyContract<'_>>, Vec<Diagnostic>) {
        let mut contracts = Vec::new();
        let mut diagnostics = Vec::new();

        for descriptor in self
            .descriptors_by_id
            .values()
            .filter(|descriptor| descriptor.implementation == ConversionImplementation::Cemt)
            .filter(|descriptor| {
                descriptor.planning_domain() == ConversionPlanningDomain::SchemaOutputProduction
            })
        {
            let (contract, mut descriptor_diagnostics) =
                conversion_output_safety_contract(descriptor);
            diagnostics.append(&mut descriptor_diagnostics);
            if let Some(contract) = contract {
                contracts.push(contract);
            }
        }

        (contracts, diagnostics)
    }
}

fn package_artifact_function_matches_output_stage(
    artifact: &ConversionPackageArtifactDescriptor,
    canonical_function_name: &str,
    formatter_profile: Option<&str>,
    color_profile: Option<&str>,
) -> bool {
    let Some(function_name) = artifact.function_name.as_deref() else {
        return false;
    };
    if function_name == canonical_function_name {
        return true;
    }
    formatter_profile
        .is_some_and(|expected| artifact.formatter_profile.as_deref() == Some(expected))
        || color_profile.is_some_and(|expected| artifact.color_profile.as_deref() == Some(expected))
}

fn package_artifact_selection_label(artifact: &ConversionPackageArtifactDescriptor) -> String {
    let function_name = artifact.function_name.as_deref().unwrap_or("<unnamed>");
    format!("`{function_name}` at `{}`", artifact.path)
}

pub fn conversion_output_safety_contract(
    descriptor: &ConversionDescriptor,
) -> (Option<ConversionOutputSafetyContract<'_>>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let output_contract = &descriptor.output_contract;

    let Some(output_syntax) = output_contract.output_syntax else {
        diagnostics.push(conversion_output_safety_diagnostic(
            CONVERSION_OUTPUT_SYNTAX_MISSING_CODE,
            format!(
                "converter `{}` must declare output syntax for encoded artifact safety",
                descriptor.id
            ),
        ));
        return (None, diagnostics);
    };

    let Some(category) = output_contract
        .encoding_category
        .as_deref()
        .map(str::trim)
        .filter(|category| !category.is_empty())
    else {
        diagnostics.push(conversion_output_safety_diagnostic(
            CONVERSION_OUTPUT_CATEGORY_MISSING_CODE,
            format!(
                "converter `{}` must declare an encoding category",
                descriptor.id
            ),
        ));
        return (None, diagnostics);
    };

    let expected_syntax = conversion_template_syntax_kind(output_syntax);
    let Some(category_syntax) = conversion_encoding_category_syntax(category) else {
        diagnostics.push(conversion_output_safety_diagnostic(
            CONVERSION_OUTPUT_UNSUPPORTED_CATEGORY_CODE,
            format!(
                "converter `{}` declares unsupported encoding category `{}`",
                descriptor.id, category
            ),
        ));
        return (None, diagnostics);
    };

    if category_syntax != expected_syntax {
        diagnostics.push(conversion_output_safety_diagnostic(
            CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE,
            format!(
                "converter `{}` output syntax `{}` cannot use `{}` encoding category",
                descriptor.id,
                conversion_output_syntax_selector(output_syntax),
                category
            ),
        ));
    }

    let Some(target_schema) = descriptor.to.schema.as_deref() else {
        diagnostics.push(conversion_output_safety_diagnostic(
            CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE,
            format!(
                "converter `{}` output safety requires an explicit target schema",
                descriptor.id
            ),
        ));
        return (None, diagnostics);
    };

    let target = TransformTemplateEncodingTarget::new(
        descriptor.to.content_type.clone(),
        target_schema.to_owned(),
        category.to_owned(),
    );
    let options = conversion_output_safety_options(output_contract, category);
    let syntax_rules = match target.syntax_rules(&options) {
        Ok(rules) => rules,
        Err(error) => {
            diagnostics.push(conversion_output_safety_diagnostic(
                CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE,
                format!(
                    "converter `{}` output target cannot be encoded safely: {}",
                    descriptor.id, error
                ),
            ));
            return (None, diagnostics);
        }
    };

    if syntax_rules.syntax != expected_syntax {
        diagnostics.push(conversion_output_safety_diagnostic(
            CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE,
            format!(
                "converter `{}` output syntax `{}` does not match target content type `{}` and schema `{}` resolved as `{}`",
                descriptor.id,
                conversion_output_syntax_selector(output_syntax),
                descriptor.to.content_type,
                target_schema,
                syntax_rules.syntax.as_str()
            ),
        ));
    }

    let color_output_profile =
        conversion_output_color_profile(descriptor, output_contract, expected_syntax, category)
            .map_err(|diagnostic| diagnostics.push(diagnostic))
            .ok()
            .flatten();

    if !diagnostics.is_empty() {
        return (None, diagnostics);
    }

    let produces = conversion_output_produced_kind(expected_syntax);
    let insertion_context =
        conversion_output_insertion_context(&target, output_contract, &options, produces);
    let pipeline = conversion_output_pipeline(output_contract, &options, &insertion_context);

    (
        Some(ConversionOutputSafetyContract {
            descriptor,
            target,
            options,
            syntax_rules,
            insertion_context,
            produces,
            color_output_profile,
            pipeline,
        }),
        diagnostics,
    )
}

pub fn compare_conversion_parity_outputs(
    contract: &ConversionParityContract<'_>,
    cemt_output: &Value,
    native_output: &Value,
) -> Option<Diagnostic> {
    if conversion_parity_outputs_match(contract, cemt_output, native_output) {
        return None;
    }

    Some(conversion_parity_diagnostic(
        CONVERSION_PARITY_DRIFT_CODE,
        format!(
            "CEMT converter `{}` and native converter `{}` produced different outputs under `{}` parity",
            contract.cemt.id,
            contract.native.id,
            conversion_parity_mode_selector(contract.mode)
        ),
    ))
}

pub fn load_conversion_parity_fixtures(
    descriptor: &ConversionDescriptor,
    package_root: impl AsRef<Path>,
) -> Result<Vec<ConversionParityFixture>, ConversionParityFixtureLoadError> {
    let package_root = package_root.as_ref();
    descriptor
        .parity_fixtures
        .iter()
        .map(|fixture_descriptor| {
            let fixture_path =
                conversion_parity_fixture_path(package_root, &fixture_descriptor.path);
            let bytes = std::fs::read(&fixture_path).map_err(|err| {
                ConversionParityFixtureLoadError::Read {
                    converter_id: descriptor.id.clone(),
                    fixture_id: fixture_descriptor.id.clone(),
                    path: fixture_path.display().to_string(),
                    message: err.to_string(),
                }
            })?;
            Ok(conversion_parity_fixture_from_bytes(
                fixture_descriptor,
                bytes,
            ))
        })
        .collect()
}

pub fn conversion_parity_fixture_from_bytes(
    descriptor: &ConversionParityFixtureDescriptor,
    bytes: Vec<u8>,
) -> ConversionParityFixture {
    ConversionParityFixture {
        id: descriptor.id.clone(),
        input: conversion_parity_fixture_input_value(descriptor, bytes),
        expected_diagnostics: Vec::new(),
        expected_diagnostic_codes: descriptor.expected_diagnostic_codes.clone(),
    }
}

fn execute_cemt_template_parity_fixture(
    descriptor: &ConversionDescriptor,
    fixture: &ConversionParityFixture,
    package_root: &Path,
    template_adapter_registry: &TransformTemplateAdapterRegistry,
) -> ConversionParityFixtureExecution {
    let Some(template) = descriptor.template.as_ref() else {
        return conversion_parity_fixture_execution_error(
            descriptor,
            fixture,
            "CEMT converter has no template descriptor".to_owned(),
        );
    };

    let template_identity = conversion_template_identity(template);
    let adapter = match template_adapter_registry.select_adapter(&template_identity) {
        TransformTemplateAdapterLookup::Matched(adapter) => adapter,
        TransformTemplateAdapterLookup::Ambiguous(adapter_ids) => {
            return conversion_parity_fixture_execution_error(
                descriptor,
                fixture,
                format!(
                    "template identity matched multiple adapters: {}",
                    adapter_ids.join(", ")
                ),
            );
        }
        TransformTemplateAdapterLookup::Unsupported => {
            return conversion_parity_fixture_execution_error(
                descriptor,
                fixture,
                format!(
                    "no template adapter supports content type `{}`",
                    template.content_type
                ),
            );
        }
    };

    if adapter.capability() != TransformTemplateAdapterCapability::Executable {
        return conversion_parity_fixture_execution_error(
            descriptor,
            fixture,
            format!("template adapter `{}` is selector-only", adapter.id()),
        );
    }

    let template_path = conversion_parity_fixture_path(package_root, &template.path);
    let template_bytes = match std::fs::read(&template_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return conversion_parity_fixture_execution_error(
                descriptor,
                fixture,
                format!(
                    "template `{}` could not be read: {}",
                    template_path.display(),
                    error
                ),
            );
        }
    };

    let template_input = TemplateInput {
        uri: template_path.to_string_lossy().into_owned(),
        bytes: template_bytes,
        identity: Some(template_identity),
        root_scope: ScopeConfig::default(),
    };
    let entrypoint = template
        .entrypoint
        .as_deref()
        .map(TransformTemplateEntrypoint::named)
        .unwrap_or_else(TransformTemplateEntrypoint::implicit);
    let params = BTreeMap::new();
    let data_bindings = vec!["input".to_owned()];
    let execution_policy = TransformExecutionPolicy::default();

    let compile_response = match adapter.compile(TransformTemplateCompileRequest {
        template: &template_input,
        entrypoint: &entrypoint,
        params: &params,
        data_bindings: &data_bindings,
        module_options: TransformTemplateModuleOptions::default(),
        module_preflight: TransformTemplateModulePreflight::default(),
        execution_policy,
    }) {
        Ok(response) => response,
        Err(error) => {
            return conversion_parity_fixture_execution_error(
                descriptor,
                fixture,
                error.to_string(),
            );
        }
    };
    let mut diagnostics = compile_response.diagnostics;
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return ConversionParityFixtureExecution {
            output: None,
            diagnostics,
        };
    }

    let input = match conversion_dom_projection_fixture_json_output(fixture) {
        Ok(input) => input,
        Err(message) => {
            return conversion_parity_fixture_execution_error(descriptor, fixture, message);
        }
    };
    let primary_input = TransformTemplateDataArtifact {
        artifact_id: "input".to_owned(),
        uri: None,
        identity: Some(FormatIdentity {
            content_type: Some(descriptor.from.content_type.clone()),
            schema: descriptor.from.schema.clone(),
            ..FormatIdentity::default()
        }),
        value: input,
    };
    let secondary_inputs = BTreeMap::new();
    let final_target = FormatIdentity {
        content_type: Some(descriptor.to.content_type.clone()),
        schema: descriptor.to.schema.clone(),
        ..FormatIdentity::default()
    };
    let (contract, mut contract_diagnostics) = conversion_output_safety_contract(descriptor);
    diagnostics.append(&mut contract_diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return ConversionParityFixtureExecution {
            output: None,
            diagnostics,
        };
    }
    let render_target = contract
        .as_ref()
        .map(|contract| contract.pipeline.cemt_target.format_identity())
        .unwrap_or(final_target);

    let render_response = match adapter.render(TransformTemplateRenderRequest {
        compiled: &compile_response.artifact,
        primary_input: &primary_input,
        secondary_inputs: &secondary_inputs,
        target: Some(&render_target),
        target_scope: &ScopeConfig::default(),
        execution_policy,
    }) {
        Ok(response) => response,
        Err(error) => {
            return conversion_parity_fixture_execution_error(
                descriptor,
                fixture,
                error.to_string(),
            );
        }
    };
    diagnostics.extend(render_response.diagnostics);
    let output = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        None
    } else if let Some(contract) = contract.as_ref() {
        let pipeline_execution = execute_conversion_output_pipeline(
            &contract.pipeline,
            render_response.output.value,
            render_response.output.source_map,
            render_response.output.output_spans,
            &descriptor.id,
            Some(&fixture.id),
            None,
        );
        diagnostics.extend(pipeline_execution.diagnostics);
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            None
        } else {
            pipeline_execution.output
        }
    } else {
        Some(render_response.output.value)
    };

    ConversionParityFixtureExecution {
        output,
        diagnostics,
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConversionOutputPipelineExecution {
    pub output: Option<Value>,
    pub source_map: Option<SourceMapStack>,
    pub output_spans: Vec<OutputSpan>,
    pub format_execution: Option<ConversionOutputPipelineStageExecution>,
    pub color_execution: Option<ConversionOutputPipelineStageExecution>,
    pub format_elapsed_ns: Option<u128>,
    pub color_elapsed_ns: Option<u128>,
    pub writer_elapsed_ns: Option<u128>,
    pub formatted_cem_tree: Option<TransformTemplateEncodedArtifact>,
    pub colored_cem_tree: Option<TransformTemplateEncodedArtifact>,
    pub diagnostics: Vec<Diagnostic>,
}

const CEM_TREE_FORMAT_CEMT_ADAPTER_ID: &str = "cem-tree-format-cemt";
const CEM_TREE_COLOR_CEMT_ADAPTER_ID: &str = "cem-tree-color-cemt";
const CSV_FORMAT_CEMT_ADAPTER_ID: &str = "csv-format-cemt";
const CSV_COLOR_CEMT_ADAPTER_ID: &str = "csv-color-cemt";
const CEMT_FORMATTER_COLORING_PIPELINE_PACKAGE_SOURCE_URI: &str =
    "schema-packages/cem-ml/v1/package.cem";
const CEM_TREE_FORMATTER_ARTIFACT_KIND: &str = "formatter";
const CEM_TREE_COLORIZER_ARTIFACT_KIND: &str = "colorizer";
const CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND: &str = "formatter-helper";
const CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND: &str = "colorizer-helper";
const CEM_TREE_FORMATTER_STAGE_ARTIFACT_KINDS: &[&str] = &[
    CEM_TREE_FORMATTER_ARTIFACT_KIND,
    CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND,
];
const CEM_TREE_COLORIZER_STAGE_ARTIFACT_KINDS: &[&str] = &[
    CEM_TREE_COLORIZER_ARTIFACT_KIND,
    CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND,
];

#[derive(Debug, Clone)]
struct CemTreeCemtOutputStage {
    adapter_id: &'static str,
    package_id: String,
    target: TransformTemplateEncodingTarget,
    stage_profile: Option<String>,
    template_uri: String,
    template_bytes: Vec<u8>,
    declaration_element: &'static str,
    function_kind: TransformTemplateOutputFunctionKind,
    function_name: String,
    canonical_function_name: &'static str,
    role: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct CemTreeCemtOutputStageSpec {
    adapter_id: &'static str,
    artifact_kind: &'static str,
    declaration_element: &'static str,
    function_kind: TransformTemplateOutputFunctionKind,
    function_name: &'static str,
    role: &'static str,
}

const CEM_TREE_FORMAT_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: CEM_TREE_FORMAT_CEMT_ADAPTER_ID,
    artifact_kind: CEM_TREE_FORMATTER_ARTIFACT_KIND,
    declaration_element: "{format-function",
    function_kind: TransformTemplateOutputFunctionKind::Format,
    function_name: "cem.format-tree",
    role: "formatter",
};

const CEM_TREE_COLOR_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: CEM_TREE_COLOR_CEMT_ADAPTER_ID,
    artifact_kind: CEM_TREE_COLORIZER_ARTIFACT_KIND,
    declaration_element: "{color-function",
    function_kind: TransformTemplateOutputFunctionKind::Color,
    function_name: "cem.color-tree",
    role: "colorizer",
};

const CSV_FORMAT_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: CSV_FORMAT_CEMT_ADAPTER_ID,
    artifact_kind: CEM_TREE_FORMATTER_ARTIFACT_KIND,
    declaration_element: "{format-function",
    function_kind: TransformTemplateOutputFunctionKind::Format,
    function_name: "csv.format-document",
    role: "formatter",
};

const CSV_COLOR_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: CSV_COLOR_CEMT_ADAPTER_ID,
    artifact_kind: CEM_TREE_COLORIZER_ARTIFACT_KIND,
    declaration_element: "{color-function",
    function_kind: TransformTemplateOutputFunctionKind::Color,
    function_name: "csv.color-document",
    role: "colorizer",
};

const YAML_FORMAT_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "yaml-format-cemt",
    artifact_kind: CEM_TREE_FORMATTER_ARTIFACT_KIND,
    declaration_element: "{format-function",
    function_kind: TransformTemplateOutputFunctionKind::Format,
    function_name: "yaml.format-document",
    role: "formatter",
};

const YAML_COLOR_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "yaml-color-cemt",
    artifact_kind: CEM_TREE_COLORIZER_ARTIFACT_KIND,
    declaration_element: "{color-function",
    function_kind: TransformTemplateOutputFunctionKind::Color,
    function_name: "yaml.color-document",
    role: "colorizer",
};

const JSON_FORMAT_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "json-format-cemt",
    artifact_kind: CEM_TREE_FORMATTER_ARTIFACT_KIND,
    declaration_element: "{format-function",
    function_kind: TransformTemplateOutputFunctionKind::Format,
    function_name: "json.format-document",
    role: "formatter",
};

const JSON_COLOR_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "json-color-cemt",
    artifact_kind: CEM_TREE_COLORIZER_ARTIFACT_KIND,
    declaration_element: "{color-function",
    function_kind: TransformTemplateOutputFunctionKind::Color,
    function_name: "json.color-document",
    role: "colorizer",
};

const JSON_SCHEMA_FORMAT_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "json-schema-format-cemt",
    artifact_kind: CEM_TREE_FORMATTER_ARTIFACT_KIND,
    declaration_element: "{format-function",
    function_kind: TransformTemplateOutputFunctionKind::Format,
    function_name: "json-schema.format-document",
    role: "formatter",
};

const JSON_SCHEMA_COLOR_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "json-schema-color-cemt",
    artifact_kind: CEM_TREE_COLORIZER_ARTIFACT_KIND,
    declaration_element: "{color-function",
    function_kind: TransformTemplateOutputFunctionKind::Color,
    function_name: "json-schema.color-document",
    role: "colorizer",
};

const MARKDOWN_FORMAT_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "markdown-format-cemt",
    artifact_kind: CEM_TREE_FORMATTER_ARTIFACT_KIND,
    declaration_element: "{format-function",
    function_kind: TransformTemplateOutputFunctionKind::Format,
    function_name: "markdown.format-document",
    role: "formatter",
};

const MARKDOWN_COLOR_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: "markdown-color-cemt",
    artifact_kind: CEM_TREE_COLORIZER_ARTIFACT_KIND,
    declaration_element: "{color-function",
    function_kind: TransformTemplateOutputFunctionKind::Color,
    function_name: "markdown.color-document",
    role: "colorizer",
};

#[cfg(test)]
fn cem_tree_format_cemt_stage(
    target: &TransformTemplateEncodingTarget,
    formatter_profile: Option<&str>,
) -> Result<CemTreeCemtOutputStage, String> {
    let schema_registry = SchemaRegistry::with_builtin_schemas();
    let conversion_registry = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schema_registry,
        conversion_registry: &conversion_registry,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    cem_tree_cemt_output_stage(
        &environment,
        CEM_TREE_FORMAT_CEMT_STAGE_SPEC,
        target,
        formatter_profile,
        Some(CEM_TREE_FORMAT_CEMT_STAGE_SPEC.function_name),
    )
}

#[cfg(test)]
fn cem_tree_color_cemt_stage(
    target: &TransformTemplateEncodingTarget,
    color_profile: Option<&str>,
) -> Result<CemTreeCemtOutputStage, String> {
    let schema_registry = SchemaRegistry::with_builtin_schemas();
    let conversion_registry = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schema_registry,
        conversion_registry: &conversion_registry,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    cem_tree_cemt_output_stage(
        &environment,
        CEM_TREE_COLOR_CEMT_STAGE_SPEC,
        target,
        color_profile,
        Some(CEM_TREE_COLOR_CEMT_STAGE_SPEC.function_name),
    )
}

fn cem_tree_cemt_output_stage(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    spec: CemTreeCemtOutputStageSpec,
    target: &TransformTemplateEncodingTarget,
    stage_profile: Option<&str>,
    requested_function_name: Option<&str>,
) -> Result<CemTreeCemtOutputStage, String> {
    let package_id =
        conversion_package_id_for_encoding_target(environment.schema_registry, target)?;
    let (formatter_profile, color_profile) = match spec.function_kind {
        TransformTemplateOutputFunctionKind::Format => (stage_profile, None),
        TransformTemplateOutputFunctionKind::Color => (None, stage_profile),
        TransformTemplateOutputFunctionKind::Encoding => (None, None),
    };
    let artifact = environment
        .conversion_registry
        .select_package_artifact_for_output_stage(
            &package_id,
            spec.artifact_kind,
            Some(CEM_TRANSFORM_CONTENT_TYPE),
            Some(CEM_TRANSFORM_SCHEMA_URI),
            target,
            requested_function_name,
            spec.function_name,
            formatter_profile,
            color_profile,
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "schema package `{}` does not declare a `{}` CEMT artifact for `{}` / `{}` targeting `{}` / `{}` / `{}` with profile `{}`",
                package_id,
                spec.artifact_kind,
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                target.content_type,
                target.schema,
                target.category,
                stage_profile.unwrap_or("<none>")
            )
        })?;
    let artifact_source = read_cem_tree_cemt_output_stage_artifact(environment, artifact)?;
    let function_name = artifact
        .function_name
        .as_deref()
        .unwrap_or(spec.function_name)
        .to_owned();
    Ok(CemTreeCemtOutputStage {
        adapter_id: spec.adapter_id,
        package_id,
        target: target.clone(),
        stage_profile: stage_profile.map(str::to_owned),
        template_uri: artifact_source.uri,
        template_bytes: artifact_source.bytes,
        declaration_element: spec.declaration_element,
        function_kind: spec.function_kind,
        function_name,
        canonical_function_name: spec.function_name,
        role: spec.role,
    })
}

fn conversion_package_id_for_encoding_target(
    schema_registry: &SchemaRegistry,
    target: &TransformTemplateEncodingTarget,
) -> Result<String, String> {
    let descriptor = schema_registry.schema(&target.schema).ok_or_else(|| {
        format!(
            "unknown conversion output target schema `{}`",
            target.schema
        )
    })?;
    let content_type = content_type_essence(&target.content_type);
    if !descriptor
        .content_types
        .iter()
        .any(|owned| owned.essence == content_type)
    {
        return Err(format!(
            "conversion output target content type `{}` is not owned by schema `{}`",
            target.content_type, target.schema
        ));
    }
    Ok(descriptor.package_id.clone())
}

fn read_cem_tree_cemt_output_stage_artifact(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    artifact: &ConversionPackageArtifactDescriptor,
) -> Result<ConversionPackageArtifactRead, String> {
    if let Some(reader) = environment.package_artifact_reader {
        match reader(artifact) {
            Ok(source) => return Ok(source),
            Err(error) => {
                if let Some(source) =
                    builtin_schema_package_artifact_source(&artifact.package_id, &artifact.path)
                {
                    return Ok(ConversionPackageArtifactRead {
                        uri: source.path.to_owned(),
                        bytes: source.source.as_bytes().to_vec(),
                        content_type: artifact.content_type.clone(),
                    });
                }
                return Err(error);
            }
        }
    }
    if let Some(source) =
        builtin_schema_package_artifact_source(&artifact.package_id, &artifact.path)
    {
        return Ok(ConversionPackageArtifactRead {
            uri: source.path.to_owned(),
            bytes: source.source.as_bytes().to_vec(),
            content_type: artifact.content_type.clone(),
        });
    }
    let Some(reader) = environment.package_artifact_reader else {
        return Err(format!(
            "schema package artifact `{}` has no embedded runtime source",
            artifact.path
        ));
    };
    reader(artifact)
}

fn package_artifact_matches_cem_tree_target(
    artifact: &ConversionPackageArtifactDescriptor,
    package_id: &str,
    target: &TransformTemplateEncodingTarget,
) -> bool {
    let target_content_type = content_type_essence(&target.content_type);
    artifact.package_id == package_id
        && artifact.content_type.as_deref() == Some(CEM_TRANSFORM_CONTENT_TYPE)
        && artifact.schema.as_deref() == Some(CEM_TRANSFORM_SCHEMA_URI)
        && artifact.target_content_type.as_deref() == Some(target_content_type.as_str())
        && artifact.target_schema.as_deref() == Some(target.schema.as_str())
        && artifact.target_category.as_deref() == Some(target.category.as_str())
}

fn package_artifact_output_function_kind(
    artifact_kind: &str,
) -> Option<TransformTemplateOutputFunctionKind> {
    CemtStageMetadataContract::from_artifact_kind(artifact_kind)
        .map(CemtStageMetadataContract::function_kind)
}

fn cemt_output_stage_helper_artifact_kind(
    function_kind: TransformTemplateOutputFunctionKind,
) -> Option<&'static str> {
    CemtStageMetadataContract::from_function_kind(function_kind)
        .map(CemtStageMetadataContract::helper_artifact_kind)
}

fn parse_cem_tree_cemt_output_artifact_module_options(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    artifact: &ConversionPackageArtifactDescriptor,
) -> Result<TransformTemplateModuleOptions, String> {
    let cache_key = cem_tree_cemt_output_artifact_cache_key(artifact);
    if let Some(cache) = environment.artifact_cache {
        if let Some(cached) = cache.module_options.borrow().get(&cache_key) {
            return cached.clone();
        }
    }

    let result = parse_cem_tree_cemt_output_artifact_module_options_uncached(environment, artifact);
    if let Some(cache) = environment.artifact_cache {
        cache
            .module_options
            .borrow_mut()
            .insert(cache_key, result.clone());
    }
    result
}

fn cem_tree_cemt_output_artifact_cache_key(
    artifact: &ConversionPackageArtifactDescriptor,
) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        artifact.package_id,
        artifact.kind,
        artifact.path,
        artifact.function_name.as_deref().unwrap_or_default(),
        artifact.function_profile.as_deref().unwrap_or_default()
    )
}

fn parse_cem_tree_cemt_output_artifact_module_options_uncached(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    artifact: &ConversionPackageArtifactDescriptor,
) -> Result<TransformTemplateModuleOptions, String> {
    let source =
        read_cem_tree_cemt_output_stage_artifact(environment, artifact).map_err(|error| {
            format!(
                "could not load `{}` artifact `{}`: {error}",
                artifact.kind, artifact.path
            )
        })?;
    let parse_response =
        parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
            template: TemplateInput {
                uri: source.uri,
                bytes: source.bytes,
                identity: Some(FormatIdentity {
                    content_type: artifact.content_type.clone(),
                    schema: artifact.schema.clone(),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
        });
    if let Some(diagnostic) = parse_response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(format!(
            "could not parse `{}` artifact `{}`: {}",
            artifact.kind, artifact.path, diagnostic.message
        ));
    }
    Ok(parse_response.module_options)
}

fn merge_cem_tree_cemt_helper_module_options(
    module_options: &mut TransformTemplateModuleOptions,
    helper_options: TransformTemplateModuleOptions,
    function_kind: TransformTemplateOutputFunctionKind,
    target: &TransformTemplateEncodingTarget,
) {
    let mut registered_helpers = module_options
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<BTreeSet<_>>();
    for function in helper_options.functions {
        if registered_helpers.insert(function.name.clone()) {
            module_options.functions.push(function);
        }
    }

    let mut registered = module_options
        .output_functions
        .iter()
        .map(|function| {
            (
                function.kind,
                function.name.clone(),
                function.profile.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    let target_content_type = content_type_essence(&target.content_type);

    for function in helper_options.output_functions {
        if function.kind != function_kind
            || content_type_essence(&function.content_type) != target_content_type
            || function.schema != target.schema
        {
            continue;
        }
        let key = (
            function.kind,
            function.name.clone(),
            function.profile.clone(),
        );
        if registered.insert(key) {
            module_options.output_functions.push(function);
        }
    }
}

fn load_cem_tree_cemt_output_stage_helpers(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    stage: &CemTreeCemtOutputStage,
    module_options: &mut TransformTemplateModuleOptions,
) -> Result<Vec<String>, String> {
    let Some(helper_kind) = cemt_output_stage_helper_artifact_kind(stage.function_kind) else {
        return Ok(Vec::new());
    };
    let mut loaded_paths = BTreeSet::new();
    let mut loaded_helpers = Vec::new();

    for artifact in environment.conversion_registry.package_artifacts() {
        let profile_matches = match stage.function_kind {
            TransformTemplateOutputFunctionKind::Format => artifact
                .formatter_profile
                .as_deref()
                .is_none_or(|profile| Some(profile) == stage.stage_profile.as_deref()),
            TransformTemplateOutputFunctionKind::Color => artifact
                .color_profile
                .as_deref()
                .is_none_or(|profile| Some(profile) == stage.stage_profile.as_deref()),
            TransformTemplateOutputFunctionKind::Encoding => true,
        };
        if artifact.kind != helper_kind
            || !package_artifact_matches_cem_tree_target(artifact, &stage.package_id, &stage.target)
            || !profile_matches
            || artifact.path == stage.template_uri
            || !loaded_paths.insert(artifact.path.clone())
        {
            continue;
        }

        let helper_options =
            parse_cem_tree_cemt_output_artifact_module_options(environment, artifact)?;
        merge_cem_tree_cemt_helper_module_options(
            module_options,
            helper_options,
            stage.function_kind,
            &stage.target,
        );
        loaded_helpers.push(artifact.path.clone());
    }

    Ok(loaded_helpers)
}

fn validate_cem_tree_cemt_output_stage_helper_resolution(
    stage: &CemTreeCemtOutputStage,
    module_options: &TransformTemplateModuleOptions,
    stage_function: &TransformTemplateOutputFunctionDescriptor,
    loaded_helpers: &[String],
) -> Result<(), String> {
    let Some(helper_name) = stage_function
        .body_expression
        .as_deref()
        .and_then(cemt_direct_call_function_name)
    else {
        return Ok(());
    };
    if helper_name.as_str() == stage.function_name.as_str() {
        return Ok(());
    }
    if module_options.output_functions.iter().any(|function| {
        function.kind == stage.function_kind && function.name.as_str() == helper_name.as_str()
    }) {
        return Ok(());
    }

    let helper_kind =
        cemt_output_stage_helper_artifact_kind(stage.function_kind).unwrap_or("helper");
    let profile = stage.stage_profile.as_deref().unwrap_or("<none>");
    if loaded_helpers.is_empty() {
        return Err(format!(
            "CEMT {} `{}` requires helper function `{}`, but no matching `{}` artifact was loaded for target `{}` / `{}` / `{}` with profile `{}`",
            stage.role,
            stage.function_name,
            helper_name,
            helper_kind,
            stage.target.content_type,
            stage.target.schema,
            stage.target.category,
            profile
        ));
    }

    Err(format!(
        "CEMT {} `{}` requires helper function `{}`, but loaded `{}` artifacts did not declare it: {}",
        stage.role,
        stage.function_name,
        helper_name,
        helper_kind,
        loaded_helpers.join(", ")
    ))
}

fn cemt_direct_call_function_name(expression: &str) -> Option<String> {
    let rest = expression.trim().strip_prefix("call(")?.trim_start();
    if let Some(rest) = rest.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].trim().to_owned());
    }

    let end = rest
        .find(|ch: char| ch == ',' || ch == ')' || ch.is_whitespace())
        .unwrap_or(rest.len());
    let name = rest[..end].trim();
    (!name.is_empty()).then(|| name.to_owned())
}

#[derive(Clone, Debug)]
struct CemTreeCemtOutputAdapter {
    stage: CemTreeCemtOutputStage,
}

impl TransformTemplateAdapter for CemTreeCemtOutputAdapter {
    fn id(&self) -> &'static str {
        self.stage.adapter_id
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
            .is_some_and(|content_type| {
                content_type_essence(content_type) == CEM_TRANSFORM_CONTENT_TYPE
            })
            || identity
                .schema
                .as_deref()
                .is_some_and(|schema| schema == CEM_TRANSFORM_SCHEMA_URI)
    }

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        let template = std::str::from_utf8(&request.template.bytes).map_err(|error| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                error.to_string(),
            )
        })?;
        if !template.contains(self.stage.declaration_element) {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                format!(
                    "template is not a {} CEMT {} declaration",
                    self.stage.function_name, self.stage.role
                ),
            ));
        }
        let stage_function = request
            .module_options
            .output_functions
            .iter()
            .find(|function| {
                function.kind == self.stage.function_kind
                    && function.name == self.stage.function_name.as_str()
            })
            .ok_or_else(|| {
                TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Compile,
                    format!(
                        "module options do not declare CEMT {} `{}`",
                        self.stage.role, self.stage.function_name
                    ),
                )
            })?;
        if stage_function.implementation != TransformTemplateOutputFunctionImplementation::Cemt {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                format!(
                    "module options do not declare CEMT {} `{}`",
                    self.stage.role, self.stage.function_name
                ),
            ));
        }
        if self.stage.function_name != self.stage.canonical_function_name
            && stage_function.extends.as_deref() != Some(self.stage.canonical_function_name)
        {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                format!(
                    "CEMT {} `{}` selected for canonical `{}` must extend `{}`",
                    self.stage.role,
                    self.stage.function_name,
                    self.stage.canonical_function_name,
                    self.stage.canonical_function_name
                ),
            ));
        }

        Ok(TransformTemplateCompileResponse {
            artifact: TransformTemplateCompiledArtifact::new(
                self.id(),
                self.kind(),
                request.template.uri.clone(),
                request.template.identity.clone(),
                request.entrypoint.clone(),
                serde_json::json!({ "outputFunction": self.stage.function_name }),
            )
            .with_module_options(request.module_options),
            diagnostics: Vec::new(),
        })
    }

    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        let binding = request
            .compiled
            .native_payload::<TransformTemplateEncodeBinding>()
            .ok_or_else(|| {
                TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Render,
                    format!(
                        "compiled {} artifact is missing the resolved encode binding",
                        self.stage.role
                    ),
                )
            })?;
        if binding.function.kind != self.stage.function_kind
            || binding.function.name.as_str() != self.stage.function_name.as_str()
        {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Render,
                format!(
                    "compiled {} binding cannot execute `{}`",
                    self.stage.role, binding.function.name
                ),
            ));
        }

        let Some(value) =
            execute_conversion_cem_tree_output_stage_body(self.stage.clone(), &request, binding)?
        else {
            return Err(TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Render,
                format!(
                    "CEMT {} `{}` requires a direct CEMT body",
                    self.stage.role, self.stage.function_name
                ),
            ));
        };

        Ok(TransformTemplateRenderResponse {
            output: TransformTemplateOutputArtifact {
                uri: None,
                identity: request.target.cloned(),
                value,
                source_map: None,
                output_spans: Vec::new(),
            },
            diagnostics: Vec::new(),
        })
    }
}

fn execute_conversion_cem_tree_output_stage_body(
    stage: CemTreeCemtOutputStage,
    request: &TransformTemplateRenderRequest<'_>,
    binding: &TransformTemplateEncodeBinding,
) -> TransformTemplateAdapterResult<Option<Value>> {
    let expressions = request
        .compiled
        .module_options
        .encode_expressions
        .iter()
        .filter(|expression| expression.owner.as_deref() == Some(stage.function_name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let registry = TransformTemplateOutputFunctionRegistry::from_module_options(
        &request.compiled.module_options,
    );
    let host_capabilities = BTreeSet::new();
    let mut value_bindings = BTreeMap::new();
    value_bindings.insert("subject".to_owned(), request.primary_input.value.clone());
    let mut reject_encode_facade = |body_binding: &TransformTemplateEncodeBinding,
                                    _subject: &Value| {
        Err(format!(
            "CEMT {} `{}` requires a direct CEMT body; encode(...) facade attempted to dispatch `{}`",
            stage.role, stage.function_name, body_binding.function.name
        ))
    };

    if expressions.is_empty() {
        if binding.function.body_expression.is_none() {
            return Ok(None);
        }
        let value = execute_transform_template_encode_binding(
            binding,
            &request.primary_input.value,
            &TransformTemplateEncodeEvaluationContext {
                registry: &registry,
                value_bindings: &value_bindings,
                host_capabilities: &host_capabilities,
                output_color_type: None,
                uri: Some(request.compiled.template_uri.as_str()),
            },
            &mut reject_encode_facade,
        )
        .map_err(|message| {
            TransformTemplateAdapterError::failed(
                stage.adapter_id,
                TransformTemplateAdapterExecutionPhase::Render,
                message,
            )
        })?;
        return Ok(Some(value));
    }

    let response = evaluate_transform_template_encode_expressions(
        &expressions,
        TransformTemplateEncodeEvaluationContext {
            registry: &registry,
            value_bindings: &value_bindings,
            host_capabilities: &host_capabilities,
            output_color_type: None,
            uri: Some(request.compiled.template_uri.as_str()),
        },
        reject_encode_facade,
    );
    if !response.diagnostics.is_empty() {
        let message = response
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(TransformTemplateAdapterError::failed(
            stage.adapter_id,
            TransformTemplateAdapterExecutionPhase::Render,
            message,
        ));
    }
    let [evaluated] = response.encoded.as_slice() else {
        return Err(TransformTemplateAdapterError::failed(
            stage.adapter_id,
            TransformTemplateAdapterExecutionPhase::Render,
            format!(
                "CEMT {} body for `{}` produced {} encoded artifacts; expected exactly one",
                stage.role,
                stage.function_name,
                response.encoded.len()
            ),
        ));
    };

    Ok(Some(evaluated.artifact.value.clone()))
}

fn execute_conversion_cem_tree_format_stage(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    binding: &TransformTemplateEncodeBinding,
    subject: &Value,
) -> Result<(Value, ConversionOutputPipelineStageExecution), String> {
    execute_conversion_cem_tree_output_stage(
        environment,
        cem_tree_cemt_output_stage(
            environment,
            CEM_TREE_FORMAT_CEMT_STAGE_SPEC,
            &binding.identity.target,
            binding.identity.formatter_profile.as_deref(),
            binding.options.formatter.as_deref(),
        )?,
        binding,
        subject,
    )
}

fn execute_conversion_cem_tree_color_stage(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    binding: &TransformTemplateEncodeBinding,
    subject: &Value,
) -> Result<(Value, ConversionOutputPipelineStageExecution), String> {
    execute_conversion_cem_tree_output_stage(
        environment,
        cem_tree_cemt_output_stage(
            environment,
            CEM_TREE_COLOR_CEMT_STAGE_SPEC,
            &binding.identity.target,
            binding.identity.color_profile.as_deref(),
            binding.options.colorizer.as_deref(),
        )?,
        binding,
        subject,
    )
}

fn execute_conversion_cem_tree_output_stage(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    stage: CemTreeCemtOutputStage,
    binding: &TransformTemplateEncodeBinding,
    subject: &Value,
) -> Result<(Value, ConversionOutputPipelineStageExecution), String> {
    let adapter = CemTreeCemtOutputAdapter {
        stage: stage.clone(),
    };
    let template = TemplateInput {
        uri: stage.template_uri.clone(),
        bytes: stage.template_bytes.clone(),
        identity: Some(FormatIdentity {
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        }),
        root_scope: ScopeConfig::default(),
    };
    let parse_response =
        parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
            template: template.clone(),
        });
    if !parse_response.diagnostics.is_empty() {
        return Err(parse_response
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    let mut module_options = parse_response.module_options;
    let loaded_helpers =
        load_cem_tree_cemt_output_stage_helpers(environment, &stage, &mut module_options)?;
    let mut execution_binding = binding.clone();
    let parsed_stage_function = module_options
        .output_functions
        .iter()
        .find(|function| {
            function.kind == stage.function_kind
                && function.name.as_str() == stage.function_name.as_str()
                && function.profile == execution_binding.function.profile
        })
        .cloned();
    if let Some(parsed_function) = parsed_stage_function {
        if parsed_function.body_declared {
            execution_binding.function = parsed_function;
        }
    }
    validate_cem_tree_cemt_output_stage_helper_resolution(
        &stage,
        &module_options,
        &execution_binding.function,
        &loaded_helpers,
    )?;
    module_options.output_functions.retain(|function| {
        function.kind != stage.function_kind
            || function.name.as_str() != stage.function_name.as_str()
    });
    module_options
        .output_functions
        .push(execution_binding.function.clone());
    let body_declared = module_options
        .encode_expressions
        .iter()
        .any(|expression| expression.owner.as_deref() == Some(stage.function_name.as_str()))
        || execution_binding.function.body_expression.is_some();
    let body_function_name = body_declared.then(|| stage.function_name.clone());
    let entrypoint = TransformTemplateEntrypoint::named(stage.function_name.as_str());
    let params = BTreeMap::new();
    let data_bindings = vec!["subject".to_owned()];
    let compile_response = adapter
        .compile(TransformTemplateCompileRequest {
            template: &template,
            entrypoint: &entrypoint,
            params: &params,
            data_bindings: &data_bindings,
            module_options,
            module_preflight: TransformTemplateModulePreflight::default(),
            execution_policy: TransformExecutionPolicy::default(),
        })
        .map_err(|error| error.to_string())?;
    if let Some(diagnostic) = compile_response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(diagnostic.message.clone());
    }

    let compiled = compile_response
        .artifact
        .with_native_payload(execution_binding.clone());
    let primary_input = TransformTemplateDataArtifact {
        artifact_id: "subject".to_owned(),
        uri: None,
        identity: Some(FormatIdentity {
            content_type: Some(binding.function.content_type.clone()),
            schema: Some(binding.function.schema.clone()),
            ..FormatIdentity::default()
        }),
        value: subject.clone(),
    };
    let secondary_inputs = BTreeMap::new();
    let target = binding.identity.target.format_identity();
    let render_response = adapter
        .render(TransformTemplateRenderRequest {
            compiled: &compiled,
            primary_input: &primary_input,
            secondary_inputs: &secondary_inputs,
            target: Some(&target),
            target_scope: &ScopeConfig::default(),
            execution_policy: TransformExecutionPolicy::default(),
        })
        .map_err(|error| error.to_string())?;
    if let Some(diagnostic) = render_response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(diagnostic.message.clone());
    }

    Ok((
        render_response.output.value,
        ConversionOutputPipelineStageExecution::CemtAdapter {
            adapter_id: compiled.adapter_id,
            function_name: execution_binding.function.name.clone(),
            fallback_function_name: None,
            body_function_name,
        },
    ))
}

pub fn execute_conversion_output_pipeline(
    pipeline: &ConversionOutputPipeline,
    rendered_value: Value,
    rendered_source_map: Option<SourceMapStack>,
    rendered_output_spans: Vec<OutputSpan>,
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let schema_registry = SchemaRegistry::with_builtin_schemas();
    let conversion_registry = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &schema_registry,
        conversion_registry: &conversion_registry,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    execute_conversion_output_pipeline_with_environment(
        &environment,
        pipeline,
        rendered_value,
        rendered_source_map,
        rendered_output_spans,
        converter_id,
        diagnostic_node,
        diagnostic_uri,
    )
}

pub fn cemt_formatter_coloring_pipeline_package_fixture_source() -> Result<String, String> {
    let source_ast =
        crate::transform_template::cemt_formatter_coloring_pipeline_showcase_source_ast();
    let mut pipeline = direct_html_output_pipeline();
    pipeline.cemt_options.formatter = Some("acme.showcase.format-tree".to_owned());
    pipeline.cemt_options.formatter_profile = Some("acme.showcase.format-tree".to_owned());
    pipeline.cemt_options.colorizer = Some("acme.showcase.color-tree".to_owned());
    pipeline.cemt_options.color_profile = Some("classes".to_owned());
    pipeline.cemt_insertion_context.formatter_profile =
        Some("acme.showcase.format-tree".to_owned());
    pipeline.writer_insertion_context.formatter_profile =
        Some("acme.showcase.format-tree".to_owned());

    let execution = execute_conversion_output_pipeline(
        &pipeline,
        source_ast.clone(),
        None,
        Vec::new(),
        "fixture-cemt-pipeline-package-artifacts",
        Some("output"),
        Some(CEMT_FORMATTER_COLORING_PIPELINE_PACKAGE_SOURCE_URI),
    );
    if !execution.diagnostics.is_empty() {
        return Err(execution
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    let formatted = execution
        .formatted_cem_tree
        .as_ref()
        .ok_or_else(|| "manifest CEMT pipeline did not retain formatted CEM tree".to_owned())?;
    let colored = execution
        .colored_cem_tree
        .as_ref()
        .ok_or_else(|| "manifest CEMT pipeline did not retain colored CEM tree".to_owned())?;
    let writer_output = execution
        .output
        .as_ref()
        .and_then(Value::as_str)
        .ok_or_else(|| "manifest CEMT pipeline writer output was not text".to_owned())?;
    if writer_output.is_empty() {
        return Err("manifest CEMT pipeline writer output was empty".to_owned());
    }
    let colorizer = match execution.color_execution.as_ref() {
        Some(ConversionOutputPipelineStageExecution::CemtAdapter { function_name, .. }) => {
            function_name.as_str()
        }
        Some(ConversionOutputPipelineStageExecution::CemtFallback { function_name }) => {
            return Err(format!(
                "manifest CEMT pipeline unexpectedly used fallback colorizer `{function_name}`"
            ));
        }
        None => return Err("manifest CEMT pipeline did not execute a colorizer".to_owned()),
    };

    crate::transform_template::render_cemt_formatter_coloring_pipeline_fixture(
        CEMT_FORMATTER_COLORING_PIPELINE_PACKAGE_SOURCE_URI,
        &source_ast,
        &formatted.value,
        &colored.value,
        colorizer,
    )
}

pub fn execute_conversion_output_pipeline_with_environment(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    pipeline: &ConversionOutputPipeline,
    rendered_value: Value,
    rendered_source_map: Option<SourceMapStack>,
    rendered_output_spans: Vec<OutputSpan>,
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let local_artifact_cache = ConversionOutputPipelineArtifactCache::default();
    let cached_environment = if environment.artifact_cache.is_some() {
        *environment
    } else {
        ConversionOutputPipelineEnvironment {
            schema_registry: environment.schema_registry,
            conversion_registry: environment.conversion_registry,
            package_artifact_reader: environment.package_artifact_reader,
            artifact_cache: Some(&local_artifact_cache),
        }
    };
    let environment = &cached_environment;
    let mut diagnostics = Vec::new();
    let functions = conversion_cem_tree_output_function_registry(environment, pipeline);
    let implementations = TransformTemplateEncodeImplementationRegistry::with_builtin_encoders();

    let format_request = TransformTemplateEncodeBindingRequest::new(
        rendered_value.clone(),
        pipeline.cemt_target.clone(),
    )
    .with_subject_type("cem-ast-node")
    .with_options(pipeline.cemt_options.clone());
    let format_binding = match functions
        .resolve_format_binding(&format_request, implementations.host_capabilities())
    {
        Ok(binding) => binding,
        Err(error) => {
            let mut diagnostic = error.diagnostic(diagnostic_uri);
            diagnostic.node = diagnostic_node.map(str::to_owned);
            diagnostics.push(diagnostic);
            return ConversionOutputPipelineExecution {
                output: None,
                diagnostics,
                ..ConversionOutputPipelineExecution::default()
            };
        }
    };
    let format_started = Instant::now();
    let format_result =
        execute_conversion_cem_tree_format_stage(environment, &format_binding, &rendered_value);
    let format_elapsed_ns = Some(format_started.elapsed().as_nanos());
    let (formatted_output, format_execution) = match format_result {
        Ok(output) => output,
        Err(message) => {
            diagnostics.push(output_pipeline_diagnostic(
                converter_id,
                diagnostic_node,
                diagnostic_uri,
                format!(
                    "CEMT formatter `{}` failed: {message}",
                    format_binding.function.name
                ),
            ));
            return ConversionOutputPipelineExecution {
                output: None,
                diagnostics,
                format_elapsed_ns,
                ..ConversionOutputPipelineExecution::default()
            };
        }
    };
    let format_execution = Some(format_execution);
    let formatted_artifact = format_binding.artifact_with_metadata(
        formatted_output,
        rendered_source_map,
        rendered_output_spans,
    );
    if let Err(error) = formatted_artifact
        .validate_insertion(&conversion_cem_tree_format_insertion_context(pipeline))
    {
        let mut diagnostic = error.diagnostic(diagnostic_uri);
        diagnostic.node = diagnostic_node.map(str::to_owned);
        diagnostics.push(diagnostic);
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics,
            format_execution,
            format_elapsed_ns,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    execute_conversion_output_pipeline_from_formatted_artifact(
        environment,
        pipeline,
        formatted_artifact,
        format_execution,
        format_elapsed_ns,
        diagnostics,
        converter_id,
        diagnostic_node,
        diagnostic_uri,
    )
}

pub fn execute_conversion_output_pipeline_from_formatted_cem_tree_with_environment(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    pipeline: &ConversionOutputPipeline,
    formatted_value: Value,
    formatted_source_map: Option<SourceMapStack>,
    formatted_output_spans: Vec<OutputSpan>,
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let local_artifact_cache = ConversionOutputPipelineArtifactCache::default();
    let cached_environment = if environment.artifact_cache.is_some() {
        *environment
    } else {
        ConversionOutputPipelineEnvironment {
            schema_registry: environment.schema_registry,
            conversion_registry: environment.conversion_registry,
            package_artifact_reader: environment.package_artifact_reader,
            artifact_cache: Some(&local_artifact_cache),
        }
    };
    let environment = &cached_environment;
    let mut diagnostics = Vec::new();
    if let Some(diagnostic) = conversion_output_pipeline_claimed_formatted_cem_tree_diagnostic(
        pipeline,
        &formatted_value,
        converter_id,
        diagnostic_node,
        diagnostic_uri,
    ) {
        diagnostics.push(diagnostic);
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    let formatted_artifact = conversion_output_pipeline_formatted_cem_tree_artifact(
        pipeline,
        formatted_value,
        formatted_source_map,
        formatted_output_spans,
    );
    if let Err(error) = formatted_artifact
        .validate_insertion(&conversion_cem_tree_format_insertion_context(pipeline))
    {
        let mut diagnostic = error.diagnostic(diagnostic_uri);
        diagnostic.node = diagnostic_node.map(str::to_owned);
        diagnostics.push(diagnostic);
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics,
            ..ConversionOutputPipelineExecution::default()
        };
    }

    execute_conversion_output_pipeline_from_formatted_artifact(
        environment,
        pipeline,
        formatted_artifact,
        None,
        None,
        diagnostics,
        converter_id,
        diagnostic_node,
        diagnostic_uri,
    )
}

pub trait CsvDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str>;
    fn into_cemt_subject(self) -> Value;
}

impl CsvDocumentOutputSubject for CsvDocumentAst {
    fn source_line_ending(&self) -> Option<&str> {
        self.line_ending.as_deref()
    }

    fn into_cemt_subject(self) -> Value {
        self.to_cemt_subject()
    }
}

#[derive(Debug, Clone)]
pub struct GenericDataCsvDocumentOutputSubject {
    table: Value,
    line_ending: Option<String>,
}

impl GenericDataCsvDocumentOutputSubject {
    pub fn new(ast: GenericDataDocumentAst) -> (Self, Vec<Diagnostic>) {
        let line_ending = ast.line_ending.clone();
        let (table, diagnostics) = generic_data_ast_to_csv_cemt_subject(&ast);
        (Self { table, line_ending }, diagnostics)
    }
}

impl CsvDocumentOutputSubject for GenericDataCsvDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str> {
        self.line_ending.as_deref()
    }

    fn into_cemt_subject(self) -> Value {
        self.table
    }
}

#[cfg(test)]
impl CsvDocumentOutputSubject for Value {
    fn source_line_ending(&self) -> Option<&str> {
        self.get("lineEnding").and_then(Value::as_str)
    }

    fn into_cemt_subject(self) -> Value {
        self
    }
}

pub fn execute_csv_document_output_pipeline_with_environment(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    table: impl CsvDocumentOutputSubject,
    target_scope: &ScopeConfig,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let output_color_selection = match csv_output_color_selection_for_scope(target_scope) {
        Ok(selection) => selection,
        Err(message) => return csv_output_pipeline_failed(diagnostic_uri, message),
    };
    let local_artifact_cache = ConversionOutputPipelineArtifactCache::default();
    let cached_environment = if environment.artifact_cache.is_some() {
        *environment
    } else {
        ConversionOutputPipelineEnvironment {
            schema_registry: environment.schema_registry,
            conversion_registry: environment.conversion_registry,
            package_artifact_reader: environment.package_artifact_reader,
            artifact_cache: Some(&local_artifact_cache),
        }
    };
    let environment = &cached_environment;
    let target =
        TransformTemplateEncodingTarget::new(CSV_CONTENT_TYPE, CSV_SCHEMA_URI, "csv-document");
    let formatter_name = target_scope
        .cemt_formatter
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(CSV_FORMAT_CEMT_STAGE_SPEC.function_name);
    let formatter_profile = target_scope
        .cemt_formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("compact");
    let presentation_options =
        match CsvFormatterPresentationOptions::from_options(&target_scope.cemt_formatter_options) {
            Ok(options) => options,
            Err(message) => return csv_output_pipeline_failed(diagnostic_uri, message),
        };
    let line_ending = csv_formatter_line_ending(table.source_line_ending(), &presentation_options);
    let table_subject = table.into_cemt_subject();
    let format_options = TransformTemplateEncodeOptions {
        formatter: Some(formatter_name.to_owned()),
        formatter_profile: Some(formatter_profile.to_owned()),
        formatter_options: target_scope.cemt_formatter_options.clone(),
        line_ending: line_ending.clone(),
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: formatter_profile == "compact",
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };
    let (format_stage, format_binding) = match resolve_cemt_output_stage_binding(
        environment,
        "CSV",
        CSV_FORMAT_CEMT_STAGE_SPEC,
        &target,
        Some(formatter_profile),
        Some(formatter_name),
        &table_subject,
        "csv-document",
        format_options,
    ) {
        Ok(resolved) => resolved,
        Err(message) => {
            return csv_output_pipeline_failed(diagnostic_uri, message);
        }
    };
    let format_started = Instant::now();
    let format_result = execute_conversion_cem_tree_output_stage(
        environment,
        format_stage,
        &format_binding,
        &table_subject,
    );
    let format_elapsed_ns = Some(format_started.elapsed().as_nanos());
    let (formatted_output, format_execution) = match format_result {
        Ok(output) => output,
        Err(message) => {
            return csv_output_pipeline_failed_with_timings(
                diagnostic_uri,
                format!(
                    "CEMT formatter `{}` failed: {message}",
                    format_binding.function.name
                ),
                format_elapsed_ns,
                None,
                None,
            );
        }
    };
    let format_execution = Some(format_execution);
    let formatted_artifact = format_binding.artifact_from_value(formatted_output);
    let mut formatted_context =
        TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
            &target,
            Some(TransformTemplateOutputProducedKind::CemTree),
        );
    formatted_context.formatter_profile = Some(formatter_profile.to_owned());
    formatted_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    formatted_context.canonical = Some(formatter_profile == "compact");
    formatted_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    if let Err(error) = formatted_artifact.validate_insertion(&formatted_context) {
        return csv_output_pipeline_failed_with_timings(
            diagnostic_uri,
            error.diagnostic(diagnostic_uri).message,
            format_elapsed_ns,
            None,
            None,
        );
    }

    let cemt_color_profile =
        match csv_cemt_color_profile_for_output(target_scope, output_color_selection.as_ref()) {
            Ok(profile) => profile,
            Err(message) => return csv_output_pipeline_failed(diagnostic_uri, message),
        };
    let wants_color = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
        || cemt_color_profile.is_some()
        || output_color_selection
            .as_ref()
            .is_some_and(csv_output_color_selection_requests_color);
    let mut color_elapsed_ns = None;
    let (writer_artifact, color_execution, colored_cem_tree) = if wants_color {
        let colorizer_name = target_scope
            .cemt_colorizer
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(CSV_COLOR_CEMT_STAGE_SPEC.function_name);
        let color_profile = cemt_color_profile.as_deref().unwrap_or("terminal");
        let color_options = TransformTemplateEncodeOptions {
            formatter_options: target_scope.cemt_formatter_options.clone(),
            formatter_profile: Some(formatter_profile.to_owned()),
            colorizer: Some(colorizer_name.to_owned()),
            color_profile: Some(color_profile.to_owned()),
            line_ending: line_ending.clone(),
            mode: TransformTemplateEncodedArtifactMode::Document,
            canonical: false,
            source_map_policy: TransformTemplateSourceMapPolicy::Generated,
            ..TransformTemplateEncodeOptions::default()
        };
        let (color_stage, color_binding) = match resolve_cemt_output_stage_binding(
            environment,
            "CSV",
            CSV_COLOR_CEMT_STAGE_SPEC,
            &target,
            Some(color_profile),
            Some(colorizer_name),
            &formatted_artifact.value,
            "cem-tree",
            color_options,
        ) {
            Ok(resolved) => resolved,
            Err(message) => {
                return csv_output_pipeline_failed(diagnostic_uri, message);
            }
        };
        let color_started = Instant::now();
        let color_result = execute_conversion_cem_tree_output_stage(
            environment,
            color_stage,
            &color_binding,
            &formatted_artifact.value,
        );
        color_elapsed_ns = Some(color_started.elapsed().as_nanos());
        let (colored_output, color_execution) = match color_result {
            Ok(output) => output,
            Err(message) => {
                return csv_output_pipeline_failed_with_timings(
                    diagnostic_uri,
                    format!(
                        "CEMT colorizer `{}` failed: {message}",
                        color_binding.function.name
                    ),
                    format_elapsed_ns,
                    color_elapsed_ns,
                    None,
                );
            }
        };
        let colored_artifact = color_binding.artifact_from_value(colored_output);
        let mut colored_context =
            TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                &target,
                Some(TransformTemplateOutputProducedKind::CemTree),
            );
        colored_context.formatter_profile = Some(formatter_profile.to_owned());
        colored_context.color_profile = Some(color_profile.to_owned());
        colored_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
        colored_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
        if let Err(error) = colored_artifact.validate_insertion(&colored_context) {
            return csv_output_pipeline_failed_with_timings(
                diagnostic_uri,
                error.diagnostic(diagnostic_uri).message,
                format_elapsed_ns,
                color_elapsed_ns,
                None,
            );
        }
        (
            colored_artifact.clone(),
            Some(color_execution),
            Some(colored_artifact),
        )
    } else {
        (formatted_artifact.clone(), None, None)
    };

    let mut writer_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &target,
        Some(TransformTemplateOutputProducedKind::Text),
    );
    writer_context.formatter_profile = Some(formatter_profile.to_owned());
    writer_context.color_profile = writer_artifact.identity.color_profile.clone();
    writer_context.output_color_type = output_color_selection
        .as_ref()
        .map(|selection| selection.output_color_type.clone());
    if output_color_selection
        .as_ref()
        .is_some_and(csv_output_color_selection_is_terminal)
    {
        writer_context.color_capability = output_color_selection
            .as_ref()
            .map(|selection| selection.output_color_type.clone());
    }
    writer_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    writer_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    let evaluated = TransformTemplateEvaluatedEncodeExpression {
        expression: TransformTemplateEncodeExpression {
            owner: Some("csv-direct-output".to_owned()),
            expression: "csv-direct-output writer".to_owned(),
            subject: "csv-document".to_owned(),
            subject_type: Some("cem-tree".to_owned()),
            target,
            options: TransformTemplateEncodeOptions::default(),
        },
        subject: writer_artifact.value.clone(),
        binding: TransformTemplateEncodeBinding {
            function: if wants_color {
                csv_output_function_descriptor(
                    CSV_COLOR_CEMT_STAGE_SPEC.function_name,
                    "csv-document",
                    "cem-tree",
                    TransformTemplateOutputFunctionKind::Color,
                    TransformTemplateOutputProducedKind::CemTree,
                    writer_artifact.identity.color_profile.clone(),
                )
            } else {
                csv_output_function_descriptor(
                    CSV_FORMAT_CEMT_STAGE_SPEC.function_name,
                    "csv-document",
                    "csv-document",
                    TransformTemplateOutputFunctionKind::Format,
                    TransformTemplateOutputProducedKind::CemTree,
                    Some(formatter_profile.to_owned()),
                )
            },
            subject_type: "cem-tree".to_owned(),
            identity: writer_artifact.identity.clone(),
            options: TransformTemplateEncodeOptions::default(),
        },
        artifact: writer_artifact,
    };
    let writer_started = Instant::now();
    let mut composition = compose_transform_template_encoded_text_artifacts(
        &[evaluated],
        &writer_context,
        diagnostic_uri,
    );
    let writer_elapsed_ns = Some(writer_started.elapsed().as_nanos());
    if !composition.diagnostics.is_empty() {
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics: std::mem::take(&mut composition.diagnostics),
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    match composition.artifact {
        Some(mut artifact) => {
            if output_color_selection
                .as_ref()
                .is_some_and(csv_output_color_selection_is_html)
            {
                csv_wrap_html_preview_artifact(&mut artifact, presentation_options.tab_size);
            }
            ConversionOutputPipelineExecution {
                output: Some(artifact.value),
                source_map: artifact.source_map,
                output_spans: artifact.output_spans,
                format_execution,
                color_execution,
                format_elapsed_ns,
                color_elapsed_ns,
                writer_elapsed_ns,
                formatted_cem_tree: Some(formatted_artifact),
                colored_cem_tree,
                diagnostics: Vec::new(),
            }
        }
        None => ConversionOutputPipelineExecution {
            output: None,
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        },
    }
}

fn resolve_cemt_output_stage_binding(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    diagnostic_label: &str,
    spec: CemTreeCemtOutputStageSpec,
    target: &TransformTemplateEncodingTarget,
    stage_profile: Option<&str>,
    requested_function_name: Option<&str>,
    subject: &Value,
    subject_type: &str,
    options: TransformTemplateEncodeOptions,
) -> Result<(CemTreeCemtOutputStage, TransformTemplateEncodeBinding), String> {
    let stage = cem_tree_cemt_output_stage(
        environment,
        spec,
        target,
        stage_profile,
        requested_function_name,
    )?;
    let parse_response =
        parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
            template: TemplateInput {
                uri: stage.template_uri.clone(),
                bytes: stage.template_bytes.clone(),
                identity: Some(FormatIdentity {
                    content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
        });
    if !parse_response.diagnostics.is_empty() {
        return Err(parse_response
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    let functions = parse_response
        .module_options
        .output_functions
        .iter()
        .filter(|function| {
            function.kind == spec.function_kind
                && function.name.as_str() == stage.function_name.as_str()
        })
        .cloned()
        .collect::<Vec<_>>();
    if functions.is_empty() {
        return Err(format!(
            "{diagnostic_label} `{}` artifact did not declare `{}`",
            spec.role, stage.function_name
        ));
    }
    let mut registry = TransformTemplateOutputFunctionRegistry::new();
    for function in functions {
        registry.register(function);
    }
    let request = TransformTemplateEncodeBindingRequest::new(subject.clone(), target.clone())
        .with_subject_type(subject_type)
        .with_options(options);
    let host_capabilities = BTreeSet::new();
    let binding = match spec.function_kind {
        TransformTemplateOutputFunctionKind::Format => registry
            .resolve_format_binding(&request, &host_capabilities)
            .map_err(|error| error.diagnostic(None).message)?,
        TransformTemplateOutputFunctionKind::Color => registry
            .resolve_color_binding(&request, &host_capabilities)
            .map_err(|error| error.diagnostic(None).message)?
            .into_encode_binding(),
        TransformTemplateOutputFunctionKind::Encoding => registry
            .resolve_encode_binding(&request, &host_capabilities)
            .map_err(|error| error.diagnostic(None).message)?,
    };
    Ok((stage, binding))
}

fn csv_output_color_selection(
    output_color_type: Option<&str>,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    let Some(output_color_type) = output_color_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    parse_transform_template_output_color_type(output_color_type)
        .map(Some)
        .map_err(|message| {
            format!("invalid CSV output color type `{output_color_type}`: {message}")
        })
}

fn csv_output_color_selection_for_scope(
    target_scope: &ScopeConfig,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    if let Some(selection) = csv_output_color_selection(target_scope.output_color_type.as_deref())?
    {
        return Ok(Some(selection));
    }

    let Some(color_profile) = target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
    else {
        return Ok(None);
    };

    if color_profile == "html" {
        return parse_transform_template_output_color_type("html")
            .map(Some)
            .map_err(|message| format!("invalid inferred CSV HTML output color type: {message}"));
    }

    Ok(None)
}

fn csv_output_color_selection_requests_color(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.output_color_type != "none"
}

fn csv_output_color_selection_is_html(selection: &TransformTemplateOutputColorSelection) -> bool {
    selection.target.category == "html-color"
        && csv_output_color_selection_requests_color(selection)
}

fn csv_output_color_selection_is_terminal(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "terminal-color"
        && csv_output_color_selection_requests_color(selection)
}

fn csv_cemt_color_profile_for_output(
    target_scope: &ScopeConfig,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> Result<Option<String>, String> {
    let explicit = target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty());
    let inferred = output_color_selection
        .filter(|selection| csv_output_color_selection_requests_color(selection))
        .and_then(|selection| match selection.target.category.as_str() {
            "html-color" => Some("html"),
            "terminal-color" => Some("terminal"),
            _ => None,
        });
    if let (Some(explicit), Some(inferred)) = (explicit, inferred) {
        if explicit != inferred {
            return Err(format!(
                "CSV color profile `{explicit}` conflicts with output color type `{}`; use `{inferred}` or omit `--cemt-color-profile`",
                output_color_selection
                    .map(|selection| selection.output_color_type.as_str())
                    .unwrap_or_default()
            ));
        }
    }
    Ok(explicit
        .map(str::to_owned)
        .or_else(|| inferred.map(str::to_owned)))
}

const CSV_HTML_PREVIEW_CLASS: &str = "cem-output-csv";

#[cfg(test)]
fn csv_html_preview_prefix(tab_size: usize) -> String {
    crate::conversion_output::html_pre_container_prefix(CSV_HTML_PREVIEW_CLASS, tab_size)
}

fn csv_wrap_html_preview_artifact(
    artifact: &mut TransformTemplateEncodedArtifact,
    tab_size: usize,
) {
    wrap_html_pre_container_artifact(artifact, CSV_HTML_PREVIEW_CLASS, tab_size);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CsvFormatterPresentationOptions {
    line_ending: Option<FormatterLineEndingMode>,
    tab_size: usize,
}

impl CsvFormatterPresentationOptions {
    fn from_options(options: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut parsed = Self {
            line_ending: None,
            tab_size: default_formatter_tab_size(),
        };
        for (key, value) in options {
            match key.as_str() {
                "csv.maxFieldWidth" => {
                    let width = value.parse::<usize>().map_err(|_| {
                        format!("CSV formatter option `{key}` must be a positive integer")
                    })?;
                    if width == 0 {
                        return Err(format!(
                            "CSV formatter option `{key}` must be greater than zero"
                        ));
                    }
                }
                "csv.stringTrim" => {
                    if !matches!(value.as_str(), "right" | "middle" | "left") {
                        return Err(format!(
                            "CSV formatter option `{key}` must be `right`, `middle`, or `left`"
                        ));
                    }
                }
                "lineEnding" => {
                    parsed.line_ending = Some(parse_formatter_line_ending_option(key, value)?);
                }
                "tabSize" => {
                    parsed.tab_size = parse_positive_formatter_usize_option(key, value)?;
                }
                _ if key.starts_with("csv.") => {
                    return Err(format!("unsupported CSV formatter option `{key}`"));
                }
                _ => {}
            }
        }
        Ok(parsed)
    }
}

fn csv_formatter_line_ending(
    source_line_ending: Option<&str>,
    options: &CsvFormatterPresentationOptions,
) -> Option<String> {
    resolve_formatter_line_ending(source_line_ending, options.line_ending)
}

fn csv_output_function_descriptor(
    name: &str,
    category: &str,
    subject: &str,
    kind: TransformTemplateOutputFunctionKind,
    produces: TransformTemplateOutputProducedKind,
    profile: Option<String>,
) -> TransformTemplateOutputFunctionDescriptor {
    cemt_output_function_descriptor(CemtOutputFunctionDescriptorSpec {
        owner: "csv",
        name,
        category,
        subject,
        kind,
        produces,
        content_type: CSV_CONTENT_TYPE,
        schema: CSV_SCHEMA_URI,
        canonical: false,
        profile,
    })
}

fn csv_output_pipeline_failed(
    diagnostic_uri: Option<&str>,
    message: String,
) -> ConversionOutputPipelineExecution {
    csv_output_pipeline_failed_with_timings(diagnostic_uri, message, None, None, None)
}

fn csv_output_pipeline_failed_with_timings(
    diagnostic_uri: Option<&str>,
    message: String,
    format_elapsed_ns: Option<u128>,
    color_elapsed_ns: Option<u128>,
    writer_elapsed_ns: Option<u128>,
) -> ConversionOutputPipelineExecution {
    failed_pipeline_execution(
        "csv-direct-output",
        Some("csv"),
        diagnostic_uri,
        message,
        format_elapsed_ns,
        color_elapsed_ns,
        writer_elapsed_ns,
    )
}

pub trait YamlDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str>;
    fn into_cemt_subject(self) -> Value;
}

impl YamlDocumentOutputSubject for YamlDocumentAst {
    fn source_line_ending(&self) -> Option<&str> {
        self.line_ending.as_deref()
    }

    fn into_cemt_subject(self) -> Value {
        self.to_cemt_subject()
    }
}

#[derive(Debug, Clone)]
pub struct GenericDataYamlDocumentOutputSubject {
    ast: GenericDataDocumentAst,
}

impl GenericDataYamlDocumentOutputSubject {
    pub fn new(ast: GenericDataDocumentAst) -> Self {
        Self { ast }
    }
}

impl YamlDocumentOutputSubject for GenericDataYamlDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str> {
        self.ast.source_line_ending()
    }

    fn into_cemt_subject(self) -> Value {
        generic_data_ast_to_yaml_cemt_subject(&self.ast)
    }
}

#[cfg(test)]
impl YamlDocumentOutputSubject for Value {
    fn source_line_ending(&self) -> Option<&str> {
        self.get("lineEnding").and_then(Value::as_str)
    }

    fn into_cemt_subject(self) -> Value {
        self
    }
}

pub fn execute_yaml_document_output_pipeline_with_environment(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    document: impl YamlDocumentOutputSubject,
    target_scope: &ScopeConfig,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let formatter_name = match yaml_formatter_name_for_scope(target_scope) {
        Ok(name) => name,
        Err(message) => return yaml_output_pipeline_failed(diagnostic_uri, message),
    };
    let formatter_profile = match yaml_formatter_profile_for_scope(target_scope) {
        Ok(profile) => profile,
        Err(message) => return yaml_output_pipeline_failed(diagnostic_uri, message),
    };
    let presentation_options = match YamlFormatterPresentationOptions::from_options(
        &target_scope.cemt_formatter_options,
    ) {
        Ok(options) => options,
        Err(message) => return yaml_output_pipeline_failed(diagnostic_uri, message),
    };
    let output_color_selection = match yaml_output_color_selection_for_scope(target_scope) {
        Ok(selection) => selection,
        Err(message) => return yaml_output_pipeline_failed(diagnostic_uri, message),
    };
    let line_ending =
        yaml_formatter_line_ending(document.source_line_ending(), &presentation_options);
    let document_subject = document.into_cemt_subject();
    let local_artifact_cache = ConversionOutputPipelineArtifactCache::default();
    let cached_environment = if environment.artifact_cache.is_some() {
        *environment
    } else {
        ConversionOutputPipelineEnvironment {
            schema_registry: environment.schema_registry,
            conversion_registry: environment.conversion_registry,
            package_artifact_reader: environment.package_artifact_reader,
            artifact_cache: Some(&local_artifact_cache),
        }
    };
    let environment = &cached_environment;
    let target =
        TransformTemplateEncodingTarget::new(YAML_CONTENT_TYPE, YAML_SCHEMA_URI, "yaml-document");
    let format_options = TransformTemplateEncodeOptions {
        formatter: Some(formatter_name.clone()),
        formatter_profile: Some(formatter_profile.clone()),
        formatter_options: target_scope.cemt_formatter_options.clone(),
        line_ending: line_ending.clone(),
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: formatter_profile == "compact",
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };
    let (format_stage, format_binding) = match resolve_cemt_output_stage_binding(
        environment,
        "YAML",
        YAML_FORMAT_CEMT_STAGE_SPEC,
        &target,
        Some(formatter_profile.as_str()),
        Some(formatter_name.as_str()),
        &document_subject,
        "yaml-document",
        format_options,
    ) {
        Ok(resolved) => resolved,
        Err(message) => return yaml_output_pipeline_failed(diagnostic_uri, message),
    };
    let format_started = Instant::now();
    let format_result = execute_conversion_cem_tree_output_stage(
        environment,
        format_stage,
        &format_binding,
        &document_subject,
    );
    let format_elapsed_ns = Some(format_started.elapsed().as_nanos());
    let (formatted_output, format_execution) = match format_result {
        Ok(output) => output,
        Err(message) => {
            return yaml_output_pipeline_failed_with_timings(
                diagnostic_uri,
                format!(
                    "CEMT formatter `{}` failed: {message}",
                    format_binding.function.name
                ),
                format_elapsed_ns,
                None,
                None,
            );
        }
    };
    let format_execution = Some(format_execution);
    let formatted_artifact = format_binding.artifact_from_value(formatted_output);
    let mut formatted_context =
        TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
            &target,
            Some(TransformTemplateOutputProducedKind::CemTree),
        );
    formatted_context.formatter_profile = formatted_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    formatted_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    formatted_context.canonical = Some(formatter_profile == "compact");
    formatted_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    if let Err(error) = formatted_artifact.validate_insertion(&formatted_context) {
        return yaml_output_pipeline_failed_with_timings(
            diagnostic_uri,
            error.diagnostic(diagnostic_uri).message,
            format_elapsed_ns,
            None,
            None,
        );
    }

    let cemt_color_profile =
        match yaml_cemt_color_profile_for_output(target_scope, output_color_selection.as_ref()) {
            Ok(profile) => profile,
            Err(message) => return yaml_output_pipeline_failed(diagnostic_uri, message),
        };
    let wants_color = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
        || cemt_color_profile.is_some()
        || output_color_selection
            .as_ref()
            .is_some_and(yaml_output_color_selection_requests_color);
    let mut color_elapsed_ns = None;
    let (writer_artifact, color_execution, colored_cem_tree) = if wants_color {
        let colorizer_name = target_scope
            .cemt_colorizer
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(YAML_COLOR_CEMT_STAGE_SPEC.function_name);
        let color_profile = cemt_color_profile.as_deref().unwrap_or("terminal");
        let color_options = TransformTemplateEncodeOptions {
            formatter_options: target_scope.cemt_formatter_options.clone(),
            formatter_profile: formatted_artifact
                .identity
                .formatter_profile
                .clone()
                .or_else(|| Some(formatter_profile.clone())),
            colorizer: Some(colorizer_name.to_owned()),
            color_profile: Some(color_profile.to_owned()),
            line_ending: line_ending.clone(),
            mode: TransformTemplateEncodedArtifactMode::Document,
            canonical: false,
            source_map_policy: TransformTemplateSourceMapPolicy::Generated,
            ..TransformTemplateEncodeOptions::default()
        };
        let (color_stage, color_binding) = match resolve_cemt_output_stage_binding(
            environment,
            "YAML",
            YAML_COLOR_CEMT_STAGE_SPEC,
            &target,
            Some(color_profile),
            Some(colorizer_name),
            &formatted_artifact.value,
            "cem-tree",
            color_options,
        ) {
            Ok(resolved) => resolved,
            Err(message) => return yaml_output_pipeline_failed(diagnostic_uri, message),
        };
        let color_started = Instant::now();
        let color_result = execute_conversion_cem_tree_output_stage(
            environment,
            color_stage,
            &color_binding,
            &formatted_artifact.value,
        );
        color_elapsed_ns = Some(color_started.elapsed().as_nanos());
        let (colored_output, color_execution) = match color_result {
            Ok(output) => output,
            Err(message) => {
                return yaml_output_pipeline_failed_with_timings(
                    diagnostic_uri,
                    format!(
                        "CEMT colorizer `{}` failed: {message}",
                        color_binding.function.name
                    ),
                    format_elapsed_ns,
                    color_elapsed_ns,
                    None,
                );
            }
        };
        let colored_artifact = color_binding.artifact_from_value(colored_output);
        let mut colored_context =
            TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                &target,
                Some(TransformTemplateOutputProducedKind::CemTree),
            );
        colored_context.formatter_profile = colored_artifact
            .identity
            .formatter_profile
            .clone()
            .or_else(|| Some(formatter_profile.clone()));
        colored_context.color_profile = Some(color_profile.to_owned());
        colored_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
        colored_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
        if let Err(error) = colored_artifact.validate_insertion(&colored_context) {
            return yaml_output_pipeline_failed_with_timings(
                diagnostic_uri,
                error.diagnostic(diagnostic_uri).message,
                format_elapsed_ns,
                color_elapsed_ns,
                None,
            );
        }
        (
            colored_artifact.clone(),
            Some(color_execution),
            Some(colored_artifact),
        )
    } else {
        (formatted_artifact.clone(), None, None)
    };

    let wrap_html_output = output_color_selection
        .as_ref()
        .is_some_and(yaml_output_color_selection_is_html)
        || writer_artifact.identity.color_profile.as_deref() == Some("html");
    let mut writer_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &target,
        Some(TransformTemplateOutputProducedKind::Text),
    );
    writer_context.formatter_profile = writer_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    writer_context.color_profile = writer_artifact.identity.color_profile.clone();
    writer_context.output_color_type = output_color_selection
        .as_ref()
        .map(|selection| selection.output_color_type.clone());
    if output_color_selection
        .as_ref()
        .is_some_and(yaml_output_color_selection_is_terminal)
    {
        writer_context.color_capability = output_color_selection
            .as_ref()
            .map(|selection| selection.output_color_type.clone());
    }
    writer_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    writer_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    let evaluated = TransformTemplateEvaluatedEncodeExpression {
        expression: TransformTemplateEncodeExpression {
            owner: Some("yaml-direct-output".to_owned()),
            expression: "yaml-direct-output writer".to_owned(),
            subject: "yaml-document".to_owned(),
            subject_type: Some("cem-tree".to_owned()),
            target,
            options: TransformTemplateEncodeOptions::default(),
        },
        subject: writer_artifact.value.clone(),
        binding: TransformTemplateEncodeBinding {
            function: yaml_output_function_descriptor(
                if color_execution.is_some() {
                    YAML_COLOR_CEMT_STAGE_SPEC.function_name
                } else {
                    YAML_FORMAT_CEMT_STAGE_SPEC.function_name
                },
                "yaml-document",
                if color_execution.is_some() {
                    "cem-tree"
                } else {
                    "yaml-document"
                },
                if color_execution.is_some() {
                    TransformTemplateOutputFunctionKind::Color
                } else {
                    TransformTemplateOutputFunctionKind::Format
                },
                TransformTemplateOutputProducedKind::CemTree,
                writer_artifact
                    .identity
                    .color_profile
                    .clone()
                    .or_else(|| writer_artifact.identity.formatter_profile.clone())
                    .or_else(|| Some(formatter_profile.clone())),
            ),
            subject_type: "cem-tree".to_owned(),
            identity: writer_artifact.identity.clone(),
            options: TransformTemplateEncodeOptions::default(),
        },
        artifact: writer_artifact,
    };
    let writer_started = Instant::now();
    let mut composition = compose_transform_template_encoded_text_artifacts(
        &[evaluated],
        &writer_context,
        diagnostic_uri,
    );
    let writer_elapsed_ns = Some(writer_started.elapsed().as_nanos());
    if !composition.diagnostics.is_empty() {
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics: std::mem::take(&mut composition.diagnostics),
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    match composition.artifact {
        Some(mut artifact) => {
            if wrap_html_output {
                yaml_wrap_html_preview_artifact(&mut artifact, presentation_options.tab_size);
            }
            ConversionOutputPipelineExecution {
                output: Some(artifact.value),
                source_map: artifact.source_map,
                output_spans: artifact.output_spans,
                format_execution,
                color_execution,
                format_elapsed_ns,
                color_elapsed_ns,
                writer_elapsed_ns,
                formatted_cem_tree: Some(formatted_artifact),
                colored_cem_tree,
                diagnostics: Vec::new(),
            }
        }
        None => ConversionOutputPipelineExecution {
            output: None,
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        },
    }
}

fn yaml_formatter_name_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let name = target_scope
        .cemt_formatter
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(YAML_FORMAT_CEMT_STAGE_SPEC.function_name);
    if name != YAML_FORMAT_CEMT_STAGE_SPEC.function_name {
        return Err(format!(
            "unsupported YAML formatter `{name}`; first-class YAML output currently supports `{}`",
            YAML_FORMAT_CEMT_STAGE_SPEC.function_name
        ));
    }
    Ok(name.to_owned())
}

fn yaml_formatter_profile_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let profile = target_scope
        .cemt_formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("compact");
    if !matches!(profile, "compact" | "pretty" | "tabular") {
        return Err(format!(
            "unsupported YAML formatter profile `{profile}`; supported profiles are compact, pretty, and tabular"
        ));
    }
    Ok(profile.to_owned())
}

fn yaml_output_color_selection(
    output_color_type: Option<&str>,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    let Some(output_color_type) = output_color_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    parse_transform_template_output_color_type(output_color_type)
        .map(Some)
        .map_err(|message| {
            format!("invalid YAML output color type `{output_color_type}`: {message}")
        })
}

fn yaml_output_color_selection_for_scope(
    target_scope: &ScopeConfig,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    yaml_output_color_selection(target_scope.output_color_type.as_deref())
}

fn yaml_output_color_selection_requests_color(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.output_color_type != "none"
}

fn yaml_output_color_selection_is_html(selection: &TransformTemplateOutputColorSelection) -> bool {
    selection.target.category == "html-color"
        && yaml_output_color_selection_requests_color(selection)
}

fn yaml_output_color_selection_is_terminal(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "terminal-color"
        && yaml_output_color_selection_requests_color(selection)
}

fn yaml_cemt_color_profile_for_output(
    target_scope: &ScopeConfig,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> Result<Option<String>, String> {
    if let Some(name) = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if name != YAML_COLOR_CEMT_STAGE_SPEC.function_name {
            return Err(format!(
                "unsupported YAML colorizer `{name}`; first-class YAML output currently supports `{}`",
                YAML_COLOR_CEMT_STAGE_SPEC.function_name
            ));
        }
    }
    let explicit = target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty());
    let explicit = match explicit {
        Some("terminal" | "html" | "md") => explicit,
        Some("none") => None,
        Some(profile) => {
            return Err(format!(
                "unsupported YAML color profile `{profile}`; supported profiles are terminal, html, md, and none"
            ));
        }
        None => None,
    };
    let inferred = output_color_selection
        .filter(|selection| yaml_output_color_selection_requests_color(selection))
        .and_then(|selection| match selection.target.category.as_str() {
            "html-color" => Some("html"),
            "terminal-color" => Some("terminal"),
            _ => None,
        });
    if let (Some(explicit), Some(inferred)) = (explicit, inferred) {
        if explicit != inferred {
            return Err(format!(
                "YAML color profile `{explicit}` conflicts with output color type `{}`; use `{inferred}` or omit `--cemt-color-profile`",
                output_color_selection
                    .map(|selection| selection.output_color_type.as_str())
                    .unwrap_or_default()
            ));
        }
    }
    Ok(explicit
        .map(str::to_owned)
        .or_else(|| inferred.map(str::to_owned)))
}

const YAML_HTML_PREVIEW_CLASS: &str = "cem-output-yaml";

#[cfg(test)]
fn yaml_html_preview_prefix(tab_size: usize) -> String {
    crate::conversion_output::html_pre_container_prefix(YAML_HTML_PREVIEW_CLASS, tab_size)
}

fn yaml_wrap_html_preview_artifact(
    artifact: &mut TransformTemplateEncodedArtifact,
    tab_size: usize,
) {
    wrap_html_pre_container_artifact(artifact, YAML_HTML_PREVIEW_CLASS, tab_size);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct YamlFormatterPresentationOptions {
    line_ending: Option<FormatterLineEndingMode>,
    tab_size: usize,
}

impl YamlFormatterPresentationOptions {
    fn from_options(options: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut parsed = Self {
            line_ending: None,
            tab_size: default_formatter_tab_size(),
        };
        for (key, value) in options {
            match key.as_str() {
                "lineEnding" => {
                    parsed.line_ending = Some(parse_formatter_line_ending_option(key, value)?);
                }
                "tabSize" => {
                    parsed.tab_size = parse_positive_formatter_usize_option(key, value)?;
                }
                _ if key.starts_with("yaml.") => {
                    return Err(format!("unsupported YAML formatter option `{key}`"));
                }
                _ => {}
            }
        }
        Ok(parsed)
    }
}

fn yaml_formatter_line_ending(
    source_line_ending: Option<&str>,
    options: &YamlFormatterPresentationOptions,
) -> Option<String> {
    resolve_formatter_line_ending(source_line_ending, options.line_ending)
}

fn yaml_output_function_descriptor(
    name: &str,
    category: &str,
    subject: &str,
    kind: TransformTemplateOutputFunctionKind,
    produces: TransformTemplateOutputProducedKind,
    profile: Option<String>,
) -> TransformTemplateOutputFunctionDescriptor {
    cemt_output_function_descriptor(CemtOutputFunctionDescriptorSpec {
        owner: "yaml",
        name,
        category,
        subject,
        kind,
        produces,
        content_type: YAML_CONTENT_TYPE,
        schema: YAML_SCHEMA_URI,
        canonical: false,
        profile,
    })
}

fn yaml_output_pipeline_failed(
    diagnostic_uri: Option<&str>,
    message: String,
) -> ConversionOutputPipelineExecution {
    yaml_output_pipeline_failed_with_timings(diagnostic_uri, message, None, None, None)
}

fn yaml_output_pipeline_failed_with_timings(
    diagnostic_uri: Option<&str>,
    message: String,
    format_elapsed_ns: Option<u128>,
    color_elapsed_ns: Option<u128>,
    writer_elapsed_ns: Option<u128>,
) -> ConversionOutputPipelineExecution {
    failed_pipeline_execution(
        "yaml-direct-output",
        Some("yaml"),
        diagnostic_uri,
        message,
        format_elapsed_ns,
        color_elapsed_ns,
        writer_elapsed_ns,
    )
}

pub trait JsonDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str>;
    fn into_cemt_subject(self) -> Value;
}

impl JsonDocumentOutputSubject for JsonDocumentAst {
    fn source_line_ending(&self) -> Option<&str> {
        self.line_ending.as_deref()
    }

    fn into_cemt_subject(self) -> Value {
        self.to_cemt_subject()
    }
}

#[derive(Debug, Clone)]
pub struct GenericDataJsonDocumentOutputSubject {
    ast: GenericDataDocumentAst,
}

impl GenericDataJsonDocumentOutputSubject {
    pub fn new(ast: GenericDataDocumentAst) -> Self {
        Self { ast }
    }
}

impl JsonDocumentOutputSubject for GenericDataJsonDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str> {
        self.ast.source_line_ending()
    }

    fn into_cemt_subject(self) -> Value {
        generic_data_ast_to_json_cemt_subject(&self.ast)
    }
}

#[cfg(test)]
impl JsonDocumentOutputSubject for Value {
    fn source_line_ending(&self) -> Option<&str> {
        self.get("lineEnding").and_then(Value::as_str)
    }

    fn into_cemt_subject(self) -> Value {
        self
    }
}

pub trait JsonSchemaDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str>;
    fn into_cemt_subject(self) -> Value;
}

impl JsonSchemaDocumentOutputSubject for JsonSchemaDocumentAst {
    fn source_line_ending(&self) -> Option<&str> {
        self.json.line_ending.as_deref()
    }

    fn into_cemt_subject(self) -> Value {
        self.to_cemt_subject()
    }
}

#[cfg(test)]
impl JsonSchemaDocumentOutputSubject for Value {
    fn source_line_ending(&self) -> Option<&str> {
        self.get("json")
            .and_then(|json| json.get("lineEnding"))
            .and_then(Value::as_str)
            .or_else(|| self.get("lineEnding").and_then(Value::as_str))
    }

    fn into_cemt_subject(self) -> Value {
        self
    }
}

pub fn execute_json_document_output_pipeline_with_environment(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    document: impl JsonDocumentOutputSubject,
    target_scope: &ScopeConfig,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let formatter_name = match json_formatter_name_for_scope(target_scope) {
        Ok(name) => name,
        Err(message) => return json_output_pipeline_failed(diagnostic_uri, message),
    };
    let formatter_profile = match json_formatter_profile_for_scope(target_scope) {
        Ok(profile) => profile,
        Err(message) => return json_output_pipeline_failed(diagnostic_uri, message),
    };
    let presentation_options = match JsonFormatterPresentationOptions::from_options(
        &target_scope.cemt_formatter_options,
    ) {
        Ok(options) => options,
        Err(message) => return json_output_pipeline_failed(diagnostic_uri, message),
    };
    let output_color_selection = match json_output_color_selection_for_scope(target_scope) {
        Ok(selection) => selection,
        Err(message) => return json_output_pipeline_failed(diagnostic_uri, message),
    };
    let line_ending =
        json_formatter_line_ending(document.source_line_ending(), &presentation_options);
    let document_subject = document.into_cemt_subject();
    let local_artifact_cache = ConversionOutputPipelineArtifactCache::default();
    let cached_environment = if environment.artifact_cache.is_some() {
        *environment
    } else {
        ConversionOutputPipelineEnvironment {
            schema_registry: environment.schema_registry,
            conversion_registry: environment.conversion_registry,
            package_artifact_reader: environment.package_artifact_reader,
            artifact_cache: Some(&local_artifact_cache),
        }
    };
    let environment = &cached_environment;
    let target = TransformTemplateEncodingTarget::new(
        JSON_CONTENT_TYPE,
        JSON_VALUE_SCHEMA_URI,
        "json-document",
    );
    let format_options = TransformTemplateEncodeOptions {
        formatter: Some(formatter_name.clone()),
        formatter_profile: Some(formatter_profile.clone()),
        formatter_options: target_scope.cemt_formatter_options.clone(),
        line_ending: line_ending.clone(),
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: formatter_profile == "compact",
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };
    let (format_stage, format_binding) = match resolve_cemt_output_stage_binding(
        environment,
        "JSON",
        JSON_FORMAT_CEMT_STAGE_SPEC,
        &target,
        Some(formatter_profile.as_str()),
        Some(formatter_name.as_str()),
        &document_subject,
        "json-document",
        format_options,
    ) {
        Ok(resolved) => resolved,
        Err(message) => return json_output_pipeline_failed(diagnostic_uri, message),
    };
    let format_started = Instant::now();
    let format_result = execute_conversion_cem_tree_output_stage(
        environment,
        format_stage,
        &format_binding,
        &document_subject,
    );
    let format_elapsed_ns = Some(format_started.elapsed().as_nanos());
    let (formatted_output, format_execution) = match format_result {
        Ok(output) => output,
        Err(message) => {
            return json_output_pipeline_failed_with_timings(
                diagnostic_uri,
                format!(
                    "CEMT formatter `{}` failed: {message}",
                    format_binding.function.name
                ),
                format_elapsed_ns,
                None,
                None,
            );
        }
    };
    let format_execution = Some(format_execution);
    let formatted_artifact = format_binding.artifact_from_value(formatted_output);
    let mut formatted_context =
        TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
            &target,
            Some(TransformTemplateOutputProducedKind::CemTree),
        );
    formatted_context.formatter_profile = formatted_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    formatted_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    formatted_context.canonical = Some(formatter_profile == "compact");
    formatted_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    if let Err(error) = formatted_artifact.validate_insertion(&formatted_context) {
        return json_output_pipeline_failed_with_timings(
            diagnostic_uri,
            error.diagnostic(diagnostic_uri).message,
            format_elapsed_ns,
            None,
            None,
        );
    }

    let cemt_color_profile =
        match json_cemt_color_profile_for_output(target_scope, output_color_selection.as_ref()) {
            Ok(profile) => profile,
            Err(message) => return json_output_pipeline_failed(diagnostic_uri, message),
        };
    let wants_color = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
        || cemt_color_profile.is_some()
        || output_color_selection
            .as_ref()
            .is_some_and(json_output_color_selection_requests_color);
    let mut color_elapsed_ns = None;
    let (writer_artifact, color_execution, colored_cem_tree) = if wants_color {
        let colorizer_name = target_scope
            .cemt_colorizer
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(JSON_COLOR_CEMT_STAGE_SPEC.function_name);
        let color_profile = cemt_color_profile.as_deref().unwrap_or("terminal");
        let color_options = TransformTemplateEncodeOptions {
            formatter_options: target_scope.cemt_formatter_options.clone(),
            formatter_profile: formatted_artifact
                .identity
                .formatter_profile
                .clone()
                .or_else(|| Some(formatter_profile.clone())),
            colorizer: Some(colorizer_name.to_owned()),
            color_profile: Some(color_profile.to_owned()),
            line_ending: line_ending.clone(),
            mode: TransformTemplateEncodedArtifactMode::Document,
            canonical: false,
            source_map_policy: TransformTemplateSourceMapPolicy::Generated,
            ..TransformTemplateEncodeOptions::default()
        };
        let (color_stage, color_binding) = match resolve_cemt_output_stage_binding(
            environment,
            "JSON",
            JSON_COLOR_CEMT_STAGE_SPEC,
            &target,
            Some(color_profile),
            Some(colorizer_name),
            &formatted_artifact.value,
            "cem-tree",
            color_options,
        ) {
            Ok(resolved) => resolved,
            Err(message) => return json_output_pipeline_failed(diagnostic_uri, message),
        };
        let color_started = Instant::now();
        let color_result = execute_conversion_cem_tree_output_stage(
            environment,
            color_stage,
            &color_binding,
            &formatted_artifact.value,
        );
        color_elapsed_ns = Some(color_started.elapsed().as_nanos());
        let (colored_output, color_execution) = match color_result {
            Ok(output) => output,
            Err(message) => {
                return json_output_pipeline_failed_with_timings(
                    diagnostic_uri,
                    format!(
                        "CEMT colorizer `{}` failed: {message}",
                        color_binding.function.name
                    ),
                    format_elapsed_ns,
                    color_elapsed_ns,
                    None,
                );
            }
        };
        let colored_artifact = color_binding.artifact_from_value(colored_output);
        let mut colored_context =
            TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                &target,
                Some(TransformTemplateOutputProducedKind::CemTree),
            );
        colored_context.formatter_profile = colored_artifact
            .identity
            .formatter_profile
            .clone()
            .or_else(|| Some(formatter_profile.clone()));
        colored_context.color_profile = Some(color_profile.to_owned());
        colored_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
        colored_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
        if let Err(error) = colored_artifact.validate_insertion(&colored_context) {
            return json_output_pipeline_failed_with_timings(
                diagnostic_uri,
                error.diagnostic(diagnostic_uri).message,
                format_elapsed_ns,
                color_elapsed_ns,
                None,
            );
        }
        (
            colored_artifact.clone(),
            Some(color_execution),
            Some(colored_artifact),
        )
    } else {
        (formatted_artifact.clone(), None, None)
    };

    let wrap_html_output = output_color_selection
        .as_ref()
        .is_some_and(json_output_color_selection_is_html)
        || writer_artifact.identity.color_profile.as_deref() == Some("html");
    let mut writer_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &target,
        Some(TransformTemplateOutputProducedKind::Text),
    );
    writer_context.formatter_profile = writer_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    writer_context.color_profile = writer_artifact.identity.color_profile.clone();
    writer_context.output_color_type = output_color_selection
        .as_ref()
        .map(|selection| selection.output_color_type.clone());
    if output_color_selection
        .as_ref()
        .is_some_and(json_output_color_selection_is_terminal)
    {
        writer_context.color_capability = output_color_selection
            .as_ref()
            .map(|selection| selection.output_color_type.clone());
    }
    writer_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    writer_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    let evaluated = TransformTemplateEvaluatedEncodeExpression {
        expression: TransformTemplateEncodeExpression {
            owner: Some("json-direct-output".to_owned()),
            expression: "json-direct-output writer".to_owned(),
            subject: "json-document".to_owned(),
            subject_type: Some("cem-tree".to_owned()),
            target,
            options: TransformTemplateEncodeOptions::default(),
        },
        subject: writer_artifact.value.clone(),
        binding: TransformTemplateEncodeBinding {
            function: json_output_function_descriptor(
                if color_execution.is_some() {
                    JSON_COLOR_CEMT_STAGE_SPEC.function_name
                } else {
                    JSON_FORMAT_CEMT_STAGE_SPEC.function_name
                },
                "json-document",
                if color_execution.is_some() {
                    "cem-tree"
                } else {
                    "json-document"
                },
                if color_execution.is_some() {
                    TransformTemplateOutputFunctionKind::Color
                } else {
                    TransformTemplateOutputFunctionKind::Format
                },
                TransformTemplateOutputProducedKind::CemTree,
                writer_artifact
                    .identity
                    .color_profile
                    .clone()
                    .or_else(|| writer_artifact.identity.formatter_profile.clone())
                    .or_else(|| Some(formatter_profile.clone())),
            ),
            subject_type: "cem-tree".to_owned(),
            identity: writer_artifact.identity.clone(),
            options: TransformTemplateEncodeOptions::default(),
        },
        artifact: writer_artifact,
    };
    let writer_started = Instant::now();
    let mut composition = compose_transform_template_encoded_text_artifacts(
        &[evaluated],
        &writer_context,
        diagnostic_uri,
    );
    let writer_elapsed_ns = Some(writer_started.elapsed().as_nanos());
    if !composition.diagnostics.is_empty() {
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics: std::mem::take(&mut composition.diagnostics),
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    match composition.artifact {
        Some(mut artifact) => {
            if wrap_html_output {
                json_wrap_html_preview_artifact(&mut artifact, presentation_options.tab_size);
            }
            ConversionOutputPipelineExecution {
                output: Some(artifact.value),
                source_map: artifact.source_map,
                output_spans: artifact.output_spans,
                format_execution,
                color_execution,
                format_elapsed_ns,
                color_elapsed_ns,
                writer_elapsed_ns,
                formatted_cem_tree: Some(formatted_artifact),
                colored_cem_tree,
                diagnostics: Vec::new(),
            }
        }
        None => ConversionOutputPipelineExecution {
            output: None,
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        },
    }
}

pub fn execute_json_schema_document_output_pipeline_with_environment(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    document: impl JsonSchemaDocumentOutputSubject,
    target_scope: &ScopeConfig,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let formatter_name = match json_schema_formatter_name_for_scope(target_scope) {
        Ok(name) => name,
        Err(message) => return json_schema_output_pipeline_failed(diagnostic_uri, message),
    };
    let formatter_profile = match json_schema_formatter_profile_for_scope(target_scope) {
        Ok(profile) => profile,
        Err(message) => return json_schema_output_pipeline_failed(diagnostic_uri, message),
    };
    let presentation_options = match JsonFormatterPresentationOptions::from_options(
        &target_scope.cemt_formatter_options,
    ) {
        Ok(options) => options,
        Err(message) => return json_schema_output_pipeline_failed(diagnostic_uri, message),
    };
    let output_color_selection = match json_schema_output_color_selection_for_scope(target_scope) {
        Ok(selection) => selection,
        Err(message) => return json_schema_output_pipeline_failed(diagnostic_uri, message),
    };
    let line_ending =
        json_formatter_line_ending(document.source_line_ending(), &presentation_options);
    let document_subject = document.into_cemt_subject();
    let local_artifact_cache = ConversionOutputPipelineArtifactCache::default();
    let cached_environment = if environment.artifact_cache.is_some() {
        *environment
    } else {
        ConversionOutputPipelineEnvironment {
            schema_registry: environment.schema_registry,
            conversion_registry: environment.conversion_registry,
            package_artifact_reader: environment.package_artifact_reader,
            artifact_cache: Some(&local_artifact_cache),
        }
    };
    let environment = &cached_environment;
    let target = TransformTemplateEncodingTarget::new(
        JSON_SCHEMA_CONTENT_TYPE,
        JSON_SCHEMA_SCHEMA_URI,
        "json-schema-document",
    );
    let format_options = TransformTemplateEncodeOptions {
        formatter: Some(formatter_name.clone()),
        formatter_profile: Some(formatter_profile.clone()),
        formatter_options: target_scope.cemt_formatter_options.clone(),
        line_ending: line_ending.clone(),
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: formatter_profile == "compact",
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };
    let (format_stage, format_binding) = match resolve_cemt_output_stage_binding(
        environment,
        "JSON Schema",
        JSON_SCHEMA_FORMAT_CEMT_STAGE_SPEC,
        &target,
        Some(formatter_profile.as_str()),
        Some(formatter_name.as_str()),
        &document_subject,
        "json-schema-document",
        format_options,
    ) {
        Ok(resolved) => resolved,
        Err(message) => return json_schema_output_pipeline_failed(diagnostic_uri, message),
    };
    let format_started = Instant::now();
    let format_result = execute_conversion_cem_tree_output_stage(
        environment,
        format_stage,
        &format_binding,
        &document_subject,
    );
    let format_elapsed_ns = Some(format_started.elapsed().as_nanos());
    let (formatted_output, format_execution) = match format_result {
        Ok(output) => output,
        Err(message) => {
            return json_schema_output_pipeline_failed_with_timings(
                diagnostic_uri,
                format!(
                    "CEMT formatter `{}` failed: {message}",
                    format_binding.function.name
                ),
                format_elapsed_ns,
                None,
                None,
            );
        }
    };
    let format_execution = Some(format_execution);
    let formatted_artifact = format_binding.artifact_from_value(formatted_output);
    let mut formatted_context =
        TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
            &target,
            Some(TransformTemplateOutputProducedKind::CemTree),
        );
    formatted_context.formatter_profile = formatted_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    formatted_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    formatted_context.canonical = Some(formatter_profile == "compact");
    formatted_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    if let Err(error) = formatted_artifact.validate_insertion(&formatted_context) {
        return json_schema_output_pipeline_failed_with_timings(
            diagnostic_uri,
            error.diagnostic(diagnostic_uri).message,
            format_elapsed_ns,
            None,
            None,
        );
    }

    let cemt_color_profile = match json_schema_cemt_color_profile_for_output(
        target_scope,
        output_color_selection.as_ref(),
    ) {
        Ok(profile) => profile,
        Err(message) => return json_schema_output_pipeline_failed(diagnostic_uri, message),
    };
    let wants_color = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
        || cemt_color_profile.is_some()
        || output_color_selection
            .as_ref()
            .is_some_and(json_schema_output_color_selection_requests_color);
    let mut color_elapsed_ns = None;
    let (writer_artifact, color_execution, colored_cem_tree) = if wants_color {
        let colorizer_name = target_scope
            .cemt_colorizer
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(JSON_SCHEMA_COLOR_CEMT_STAGE_SPEC.function_name);
        let color_profile = cemt_color_profile.as_deref().unwrap_or("terminal");
        let color_options = TransformTemplateEncodeOptions {
            formatter_options: target_scope.cemt_formatter_options.clone(),
            formatter_profile: formatted_artifact
                .identity
                .formatter_profile
                .clone()
                .or_else(|| Some(formatter_profile.clone())),
            colorizer: Some(colorizer_name.to_owned()),
            color_profile: Some(color_profile.to_owned()),
            line_ending: line_ending.clone(),
            mode: TransformTemplateEncodedArtifactMode::Document,
            canonical: false,
            source_map_policy: TransformTemplateSourceMapPolicy::Generated,
            ..TransformTemplateEncodeOptions::default()
        };
        let (color_stage, color_binding) = match resolve_cemt_output_stage_binding(
            environment,
            "JSON Schema",
            JSON_SCHEMA_COLOR_CEMT_STAGE_SPEC,
            &target,
            Some(color_profile),
            Some(colorizer_name),
            &formatted_artifact.value,
            "cem-tree",
            color_options,
        ) {
            Ok(resolved) => resolved,
            Err(message) => return json_schema_output_pipeline_failed(diagnostic_uri, message),
        };
        let color_started = Instant::now();
        let color_result = execute_conversion_cem_tree_output_stage(
            environment,
            color_stage,
            &color_binding,
            &formatted_artifact.value,
        );
        color_elapsed_ns = Some(color_started.elapsed().as_nanos());
        let (colored_output, color_execution) = match color_result {
            Ok(output) => output,
            Err(message) => {
                return json_schema_output_pipeline_failed_with_timings(
                    diagnostic_uri,
                    format!(
                        "CEMT colorizer `{}` failed: {message}",
                        color_binding.function.name
                    ),
                    format_elapsed_ns,
                    color_elapsed_ns,
                    None,
                );
            }
        };
        let colored_artifact = color_binding.artifact_from_value(colored_output);
        let mut colored_context =
            TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                &target,
                Some(TransformTemplateOutputProducedKind::CemTree),
            );
        colored_context.formatter_profile = colored_artifact
            .identity
            .formatter_profile
            .clone()
            .or_else(|| Some(formatter_profile.clone()));
        colored_context.color_profile = Some(color_profile.to_owned());
        colored_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
        colored_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
        if let Err(error) = colored_artifact.validate_insertion(&colored_context) {
            return json_schema_output_pipeline_failed_with_timings(
                diagnostic_uri,
                error.diagnostic(diagnostic_uri).message,
                format_elapsed_ns,
                color_elapsed_ns,
                None,
            );
        }
        (
            colored_artifact.clone(),
            Some(color_execution),
            Some(colored_artifact),
        )
    } else {
        (formatted_artifact.clone(), None, None)
    };

    let wrap_html_output = output_color_selection
        .as_ref()
        .is_some_and(json_schema_output_color_selection_is_html)
        || writer_artifact.identity.color_profile.as_deref() == Some("html");
    let mut writer_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &target,
        Some(TransformTemplateOutputProducedKind::Text),
    );
    writer_context.formatter_profile = writer_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    writer_context.color_profile = writer_artifact.identity.color_profile.clone();
    writer_context.output_color_type = output_color_selection
        .as_ref()
        .map(|selection| selection.output_color_type.clone());
    if output_color_selection
        .as_ref()
        .is_some_and(json_schema_output_color_selection_is_terminal)
    {
        writer_context.color_capability = output_color_selection
            .as_ref()
            .map(|selection| selection.output_color_type.clone());
    }
    writer_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    writer_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    let evaluated = TransformTemplateEvaluatedEncodeExpression {
        expression: TransformTemplateEncodeExpression {
            owner: Some("json-schema-direct-output".to_owned()),
            expression: "json-schema-direct-output writer".to_owned(),
            subject: "json-schema-document".to_owned(),
            subject_type: Some("cem-tree".to_owned()),
            target,
            options: TransformTemplateEncodeOptions::default(),
        },
        subject: writer_artifact.value.clone(),
        binding: TransformTemplateEncodeBinding {
            function: json_schema_output_function_descriptor(
                if color_execution.is_some() {
                    JSON_SCHEMA_COLOR_CEMT_STAGE_SPEC.function_name
                } else {
                    JSON_SCHEMA_FORMAT_CEMT_STAGE_SPEC.function_name
                },
                "json-schema-document",
                if color_execution.is_some() {
                    "cem-tree"
                } else {
                    "json-schema-document"
                },
                if color_execution.is_some() {
                    TransformTemplateOutputFunctionKind::Color
                } else {
                    TransformTemplateOutputFunctionKind::Format
                },
                TransformTemplateOutputProducedKind::CemTree,
                writer_artifact
                    .identity
                    .color_profile
                    .clone()
                    .or_else(|| writer_artifact.identity.formatter_profile.clone())
                    .or_else(|| Some(formatter_profile.clone())),
            ),
            subject_type: "cem-tree".to_owned(),
            identity: writer_artifact.identity.clone(),
            options: TransformTemplateEncodeOptions::default(),
        },
        artifact: writer_artifact,
    };
    let writer_started = Instant::now();
    let mut composition = compose_transform_template_encoded_text_artifacts(
        &[evaluated],
        &writer_context,
        diagnostic_uri,
    );
    let writer_elapsed_ns = Some(writer_started.elapsed().as_nanos());
    if !composition.diagnostics.is_empty() {
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics: std::mem::take(&mut composition.diagnostics),
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    match composition.artifact {
        Some(mut artifact) => {
            if wrap_html_output {
                json_schema_wrap_html_preview_artifact(
                    &mut artifact,
                    presentation_options.tab_size,
                );
            }
            ConversionOutputPipelineExecution {
                output: Some(artifact.value),
                source_map: artifact.source_map,
                output_spans: artifact.output_spans,
                format_execution,
                color_execution,
                format_elapsed_ns,
                color_elapsed_ns,
                writer_elapsed_ns,
                formatted_cem_tree: Some(formatted_artifact),
                colored_cem_tree,
                diagnostics: Vec::new(),
            }
        }
        None => ConversionOutputPipelineExecution {
            output: None,
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        },
    }
}

fn json_formatter_name_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let name = target_scope
        .cemt_formatter
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(JSON_FORMAT_CEMT_STAGE_SPEC.function_name);
    if name != JSON_FORMAT_CEMT_STAGE_SPEC.function_name {
        return Err(format!(
            "unsupported JSON formatter `{name}`; first-class JSON output currently supports `{}`",
            JSON_FORMAT_CEMT_STAGE_SPEC.function_name
        ));
    }
    Ok(name.to_owned())
}

fn json_formatter_profile_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let profile = target_scope
        .cemt_formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("compact");
    if !matches!(profile, "compact" | "pretty" | "tabular") {
        return Err(format!(
            "unsupported JSON formatter profile `{profile}`; supported profiles are compact, pretty, and tabular"
        ));
    }
    Ok(profile.to_owned())
}

fn json_output_color_selection(
    output_color_type: Option<&str>,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    let Some(output_color_type) = output_color_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    parse_transform_template_output_color_type(output_color_type)
        .map(Some)
        .map_err(|message| {
            format!("invalid JSON output color type `{output_color_type}`: {message}")
        })
}

fn json_output_color_selection_for_scope(
    target_scope: &ScopeConfig,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    json_output_color_selection(target_scope.output_color_type.as_deref())
}

fn json_output_color_selection_requests_color(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.output_color_type != "none"
}

fn json_output_color_selection_is_html(selection: &TransformTemplateOutputColorSelection) -> bool {
    selection.target.category == "html-color"
        && json_output_color_selection_requests_color(selection)
}

fn json_output_color_selection_is_terminal(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "terminal-color"
        && json_output_color_selection_requests_color(selection)
}

fn json_cemt_color_profile_for_output(
    target_scope: &ScopeConfig,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> Result<Option<String>, String> {
    if let Some(name) = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if name != JSON_COLOR_CEMT_STAGE_SPEC.function_name {
            return Err(format!(
                "unsupported JSON colorizer `{name}`; first-class JSON output currently supports `{}`",
                JSON_COLOR_CEMT_STAGE_SPEC.function_name
            ));
        }
    }
    let explicit = target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty());
    let explicit = match explicit {
        Some("terminal" | "html" | "md") => explicit,
        Some("none") => None,
        Some(profile) => {
            return Err(format!(
                "unsupported JSON color profile `{profile}`; supported profiles are terminal, html, md, and none"
            ));
        }
        None => None,
    };
    let inferred = output_color_selection
        .filter(|selection| json_output_color_selection_requests_color(selection))
        .and_then(|selection| match selection.target.category.as_str() {
            "html-color" => Some("html"),
            "terminal-color" => Some("terminal"),
            _ => None,
        });
    if let (Some(explicit), Some(inferred)) = (explicit, inferred) {
        if explicit != inferred {
            return Err(format!(
                "JSON color profile `{explicit}` conflicts with output color type `{}`; use `{inferred}` or omit `--cemt-color-profile`",
                output_color_selection
                    .map(|selection| selection.output_color_type.as_str())
                    .unwrap_or_default()
            ));
        }
    }
    Ok(explicit
        .map(str::to_owned)
        .or_else(|| inferred.map(str::to_owned)))
}

fn json_schema_formatter_name_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let name = target_scope
        .cemt_formatter
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(JSON_SCHEMA_FORMAT_CEMT_STAGE_SPEC.function_name);
    if name != JSON_SCHEMA_FORMAT_CEMT_STAGE_SPEC.function_name {
        return Err(format!(
            "unsupported JSON Schema formatter `{name}`; first-class JSON Schema output currently supports `{}`",
            JSON_SCHEMA_FORMAT_CEMT_STAGE_SPEC.function_name
        ));
    }
    Ok(name.to_owned())
}

fn json_schema_formatter_profile_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let profile = target_scope
        .cemt_formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("compact");
    if !matches!(profile, "compact" | "pretty" | "tabular") {
        return Err(format!(
            "unsupported JSON Schema formatter profile `{profile}`; supported profiles are compact, pretty, and tabular"
        ));
    }
    Ok(profile.to_owned())
}

fn json_schema_output_color_selection(
    output_color_type: Option<&str>,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    let Some(output_color_type) = output_color_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    parse_transform_template_output_color_type(output_color_type)
        .map(Some)
        .map_err(|message| {
            format!("invalid JSON Schema output color type `{output_color_type}`: {message}")
        })
}

fn json_schema_output_color_selection_for_scope(
    target_scope: &ScopeConfig,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    json_schema_output_color_selection(target_scope.output_color_type.as_deref())
}

fn json_schema_output_color_selection_requests_color(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.output_color_type != "none"
}

fn json_schema_output_color_selection_is_html(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "html-color"
        && json_schema_output_color_selection_requests_color(selection)
}

fn json_schema_output_color_selection_is_terminal(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "terminal-color"
        && json_schema_output_color_selection_requests_color(selection)
}

fn json_schema_cemt_color_profile_for_output(
    target_scope: &ScopeConfig,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> Result<Option<String>, String> {
    if let Some(name) = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if name != JSON_SCHEMA_COLOR_CEMT_STAGE_SPEC.function_name {
            return Err(format!(
                "unsupported JSON Schema colorizer `{name}`; first-class JSON Schema output currently supports `{}`",
                JSON_SCHEMA_COLOR_CEMT_STAGE_SPEC.function_name
            ));
        }
    }
    let explicit = target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty());
    let explicit = match explicit {
        Some("terminal" | "html" | "md") => explicit,
        Some("none") => None,
        Some(profile) => {
            return Err(format!(
                "unsupported JSON Schema color profile `{profile}`; supported profiles are terminal, html, md, and none"
            ));
        }
        None => None,
    };
    let inferred = output_color_selection
        .filter(|selection| json_schema_output_color_selection_requests_color(selection))
        .and_then(|selection| match selection.target.category.as_str() {
            "html-color" => Some("html"),
            "terminal-color" => Some("terminal"),
            _ => None,
        });
    if let (Some(explicit), Some(inferred)) = (explicit, inferred) {
        if explicit != inferred {
            return Err(format!(
                "JSON Schema color profile `{explicit}` conflicts with output color type `{}`; use `{inferred}` or omit `--cemt-color-profile`",
                output_color_selection
                    .map(|selection| selection.output_color_type.as_str())
                    .unwrap_or_default()
            ));
        }
    }
    Ok(explicit
        .map(str::to_owned)
        .or_else(|| inferred.map(str::to_owned)))
}

const JSON_HTML_PREVIEW_CLASS: &str = "cem-output-json";

#[cfg(test)]
fn json_html_preview_prefix(tab_size: usize) -> String {
    crate::conversion_output::html_pre_container_prefix(JSON_HTML_PREVIEW_CLASS, tab_size)
}

fn json_wrap_html_preview_artifact(
    artifact: &mut TransformTemplateEncodedArtifact,
    tab_size: usize,
) {
    wrap_html_pre_container_artifact(artifact, JSON_HTML_PREVIEW_CLASS, tab_size);
}

const JSON_SCHEMA_HTML_PREVIEW_CLASS: &str = "cem-output-json-schema";

#[cfg(test)]
fn json_schema_html_preview_prefix(tab_size: usize) -> String {
    crate::conversion_output::html_pre_container_prefix(JSON_SCHEMA_HTML_PREVIEW_CLASS, tab_size)
}

fn json_schema_wrap_html_preview_artifact(
    artifact: &mut TransformTemplateEncodedArtifact,
    tab_size: usize,
) {
    wrap_html_pre_container_artifact(artifact, JSON_SCHEMA_HTML_PREVIEW_CLASS, tab_size);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JsonFormatterPresentationOptions {
    line_ending: Option<FormatterLineEndingMode>,
    tab_size: usize,
}

impl JsonFormatterPresentationOptions {
    fn from_options(options: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut parsed = Self {
            line_ending: None,
            tab_size: default_formatter_tab_size(),
        };
        for (key, value) in options {
            match key.as_str() {
                "lineEnding" => {
                    parsed.line_ending = Some(parse_formatter_line_ending_option(key, value)?);
                }
                "tabSize" => {
                    parsed.tab_size = parse_positive_formatter_usize_option(key, value)?;
                }
                _ if key.starts_with("json.") => {
                    return Err(format!("unsupported JSON formatter option `{key}`"));
                }
                _ => {}
            }
        }
        Ok(parsed)
    }
}

fn json_formatter_line_ending(
    source_line_ending: Option<&str>,
    options: &JsonFormatterPresentationOptions,
) -> Option<String> {
    resolve_formatter_line_ending(source_line_ending, options.line_ending)
}

fn json_output_function_descriptor(
    name: &str,
    category: &str,
    subject: &str,
    kind: TransformTemplateOutputFunctionKind,
    produces: TransformTemplateOutputProducedKind,
    profile: Option<String>,
) -> TransformTemplateOutputFunctionDescriptor {
    cemt_output_function_descriptor(CemtOutputFunctionDescriptorSpec {
        owner: "json",
        name,
        category,
        subject,
        kind,
        produces,
        content_type: JSON_CONTENT_TYPE,
        schema: JSON_VALUE_SCHEMA_URI,
        canonical: false,
        profile,
    })
}

fn json_output_pipeline_failed(
    diagnostic_uri: Option<&str>,
    message: String,
) -> ConversionOutputPipelineExecution {
    json_output_pipeline_failed_with_timings(diagnostic_uri, message, None, None, None)
}

fn json_output_pipeline_failed_with_timings(
    diagnostic_uri: Option<&str>,
    message: String,
    format_elapsed_ns: Option<u128>,
    color_elapsed_ns: Option<u128>,
    writer_elapsed_ns: Option<u128>,
) -> ConversionOutputPipelineExecution {
    failed_pipeline_execution(
        "json-direct-output",
        Some("json"),
        diagnostic_uri,
        message,
        format_elapsed_ns,
        color_elapsed_ns,
        writer_elapsed_ns,
    )
}

fn json_schema_output_function_descriptor(
    name: &str,
    category: &str,
    subject: &str,
    kind: TransformTemplateOutputFunctionKind,
    produces: TransformTemplateOutputProducedKind,
    profile: Option<String>,
) -> TransformTemplateOutputFunctionDescriptor {
    cemt_output_function_descriptor(CemtOutputFunctionDescriptorSpec {
        owner: "json-schema",
        name,
        category,
        subject,
        kind,
        produces,
        content_type: JSON_SCHEMA_CONTENT_TYPE,
        schema: JSON_SCHEMA_SCHEMA_URI,
        canonical: false,
        profile,
    })
}

fn json_schema_output_pipeline_failed(
    diagnostic_uri: Option<&str>,
    message: String,
) -> ConversionOutputPipelineExecution {
    json_schema_output_pipeline_failed_with_timings(diagnostic_uri, message, None, None, None)
}

fn json_schema_output_pipeline_failed_with_timings(
    diagnostic_uri: Option<&str>,
    message: String,
    format_elapsed_ns: Option<u128>,
    color_elapsed_ns: Option<u128>,
    writer_elapsed_ns: Option<u128>,
) -> ConversionOutputPipelineExecution {
    failed_pipeline_execution(
        "json-schema-direct-output",
        Some("json-schema"),
        diagnostic_uri,
        message,
        format_elapsed_ns,
        color_elapsed_ns,
        writer_elapsed_ns,
    )
}

pub trait MarkdownDocumentOutputSubject {
    fn source_line_ending(&self) -> Option<&str>;
    fn into_cemt_subject(self) -> Value;
}

impl MarkdownDocumentOutputSubject for MarkdownDocumentAst {
    fn source_line_ending(&self) -> Option<&str> {
        self.line_ending.as_deref()
    }

    fn into_cemt_subject(self) -> Value {
        self.to_cemt_subject()
    }
}

#[cfg(test)]
impl MarkdownDocumentOutputSubject for Value {
    fn source_line_ending(&self) -> Option<&str> {
        self.get("lineEnding").and_then(Value::as_str)
    }

    fn into_cemt_subject(self) -> Value {
        self
    }
}

pub fn execute_markdown_document_output_pipeline_with_environment(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    document: impl MarkdownDocumentOutputSubject,
    target_scope: &ScopeConfig,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let formatter_name = match markdown_formatter_name_for_scope(target_scope) {
        Ok(name) => name,
        Err(message) => return markdown_output_pipeline_failed(diagnostic_uri, message),
    };
    let formatter_profile = match markdown_formatter_profile_for_scope(target_scope) {
        Ok(profile) => profile,
        Err(message) => return markdown_output_pipeline_failed(diagnostic_uri, message),
    };
    let presentation_options = match MarkdownFormatterPresentationOptions::from_options(
        &target_scope.cemt_formatter_options,
    ) {
        Ok(options) => options,
        Err(message) => return markdown_output_pipeline_failed(diagnostic_uri, message),
    };
    let output_color_selection = match markdown_output_color_selection_for_scope(target_scope) {
        Ok(selection) => selection,
        Err(message) => return markdown_output_pipeline_failed(diagnostic_uri, message),
    };
    let line_ending =
        markdown_formatter_line_ending(document.source_line_ending(), &presentation_options);
    let document_subject = document.into_cemt_subject();
    let local_artifact_cache = ConversionOutputPipelineArtifactCache::default();
    let cached_environment = if environment.artifact_cache.is_some() {
        *environment
    } else {
        ConversionOutputPipelineEnvironment {
            schema_registry: environment.schema_registry,
            conversion_registry: environment.conversion_registry,
            package_artifact_reader: environment.package_artifact_reader,
            artifact_cache: Some(&local_artifact_cache),
        }
    };
    let environment = &cached_environment;
    let target = TransformTemplateEncodingTarget::new(
        MARKDOWN_CONTENT_TYPE,
        MARKDOWN_SCHEMA_URI,
        "markdown-document",
    );
    let format_options = TransformTemplateEncodeOptions {
        formatter: Some(formatter_name.clone()),
        formatter_profile: Some(formatter_profile.clone()),
        formatter_options: target_scope.cemt_formatter_options.clone(),
        line_ending: line_ending.clone(),
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: formatter_profile == "compact",
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };
    let (format_stage, format_binding) = match resolve_cemt_output_stage_binding(
        environment,
        "Markdown",
        MARKDOWN_FORMAT_CEMT_STAGE_SPEC,
        &target,
        Some(formatter_profile.as_str()),
        Some(formatter_name.as_str()),
        &document_subject,
        "markdown-document",
        format_options,
    ) {
        Ok(resolved) => resolved,
        Err(message) => return markdown_output_pipeline_failed(diagnostic_uri, message),
    };
    let format_started = Instant::now();
    let format_result = execute_conversion_cem_tree_output_stage(
        environment,
        format_stage,
        &format_binding,
        &document_subject,
    );
    let format_elapsed_ns = Some(format_started.elapsed().as_nanos());
    let (formatted_output, format_execution) = match format_result {
        Ok(output) => output,
        Err(message) => {
            return markdown_output_pipeline_failed_with_timings(
                diagnostic_uri,
                format!(
                    "CEMT formatter `{}` failed: {message}",
                    format_binding.function.name
                ),
                format_elapsed_ns,
                None,
                None,
            );
        }
    };
    let format_execution = Some(format_execution);
    let formatted_artifact = format_binding.artifact_from_value(formatted_output);
    let mut formatted_context =
        TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
            &target,
            Some(TransformTemplateOutputProducedKind::CemTree),
        );
    formatted_context.formatter_profile = formatted_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    formatted_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    formatted_context.canonical = Some(formatter_profile == "compact");
    formatted_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    if let Err(error) = formatted_artifact.validate_insertion(&formatted_context) {
        return markdown_output_pipeline_failed_with_timings(
            diagnostic_uri,
            error.diagnostic(diagnostic_uri).message,
            format_elapsed_ns,
            None,
            None,
        );
    }

    let cemt_color_profile =
        match markdown_cemt_color_profile_for_output(target_scope, output_color_selection.as_ref())
        {
            Ok(profile) => profile,
            Err(message) => return markdown_output_pipeline_failed(diagnostic_uri, message),
        };
    let wants_color = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
        || cemt_color_profile.is_some()
        || output_color_selection
            .as_ref()
            .is_some_and(markdown_output_color_selection_requests_color);
    let mut color_elapsed_ns = None;
    let (writer_artifact, color_execution, colored_cem_tree) = if wants_color {
        let colorizer_name = target_scope
            .cemt_colorizer
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(MARKDOWN_COLOR_CEMT_STAGE_SPEC.function_name);
        let color_profile = cemt_color_profile.as_deref().unwrap_or("terminal");
        let color_options = TransformTemplateEncodeOptions {
            formatter_options: target_scope.cemt_formatter_options.clone(),
            formatter_profile: formatted_artifact
                .identity
                .formatter_profile
                .clone()
                .or_else(|| Some(formatter_profile.clone())),
            colorizer: Some(colorizer_name.to_owned()),
            color_profile: Some(color_profile.to_owned()),
            line_ending: line_ending.clone(),
            mode: TransformTemplateEncodedArtifactMode::Document,
            canonical: false,
            source_map_policy: TransformTemplateSourceMapPolicy::Generated,
            ..TransformTemplateEncodeOptions::default()
        };
        let (color_stage, color_binding) = match resolve_cemt_output_stage_binding(
            environment,
            "Markdown",
            MARKDOWN_COLOR_CEMT_STAGE_SPEC,
            &target,
            Some(color_profile),
            Some(colorizer_name),
            &formatted_artifact.value,
            "cem-tree",
            color_options,
        ) {
            Ok(resolved) => resolved,
            Err(message) => return markdown_output_pipeline_failed(diagnostic_uri, message),
        };
        let color_started = Instant::now();
        let color_result = execute_conversion_cem_tree_output_stage(
            environment,
            color_stage,
            &color_binding,
            &formatted_artifact.value,
        );
        color_elapsed_ns = Some(color_started.elapsed().as_nanos());
        let (colored_output, color_execution) = match color_result {
            Ok(output) => output,
            Err(message) => {
                return markdown_output_pipeline_failed_with_timings(
                    diagnostic_uri,
                    format!(
                        "CEMT colorizer `{}` failed: {message}",
                        color_binding.function.name
                    ),
                    format_elapsed_ns,
                    color_elapsed_ns,
                    None,
                );
            }
        };
        let colored_artifact = color_binding.artifact_from_value(colored_output);
        let mut colored_context =
            TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                &target,
                Some(TransformTemplateOutputProducedKind::CemTree),
            );
        colored_context.formatter_profile = colored_artifact
            .identity
            .formatter_profile
            .clone()
            .or_else(|| Some(formatter_profile.clone()));
        colored_context.color_profile = Some(color_profile.to_owned());
        colored_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
        colored_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
        if let Err(error) = colored_artifact.validate_insertion(&colored_context) {
            return markdown_output_pipeline_failed_with_timings(
                diagnostic_uri,
                error.diagnostic(diagnostic_uri).message,
                format_elapsed_ns,
                color_elapsed_ns,
                None,
            );
        }
        (
            colored_artifact.clone(),
            Some(color_execution),
            Some(colored_artifact),
        )
    } else {
        (formatted_artifact.clone(), None, None)
    };

    let wrap_html_output = output_color_selection
        .as_ref()
        .is_some_and(markdown_output_color_selection_is_html)
        || writer_artifact.identity.color_profile.as_deref() == Some("html");
    let mut writer_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &target,
        Some(TransformTemplateOutputProducedKind::Text),
    );
    writer_context.formatter_profile = writer_artifact
        .identity
        .formatter_profile
        .clone()
        .or_else(|| Some(formatter_profile.clone()));
    writer_context.color_profile = writer_artifact.identity.color_profile.clone();
    writer_context.output_color_type = output_color_selection
        .as_ref()
        .map(|selection| selection.output_color_type.clone());
    if output_color_selection
        .as_ref()
        .is_some_and(markdown_output_color_selection_is_terminal)
    {
        writer_context.color_capability = output_color_selection
            .as_ref()
            .map(|selection| selection.output_color_type.clone());
    }
    writer_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    writer_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    let evaluated = TransformTemplateEvaluatedEncodeExpression {
        expression: TransformTemplateEncodeExpression {
            owner: Some("markdown-direct-output".to_owned()),
            expression: "markdown-direct-output writer".to_owned(),
            subject: "markdown-document".to_owned(),
            subject_type: Some("cem-tree".to_owned()),
            target,
            options: TransformTemplateEncodeOptions::default(),
        },
        subject: writer_artifact.value.clone(),
        binding: TransformTemplateEncodeBinding {
            function: markdown_output_function_descriptor(
                if color_execution.is_some() {
                    MARKDOWN_COLOR_CEMT_STAGE_SPEC.function_name
                } else {
                    MARKDOWN_FORMAT_CEMT_STAGE_SPEC.function_name
                },
                "markdown-document",
                if color_execution.is_some() {
                    "cem-tree"
                } else {
                    "markdown-document"
                },
                if color_execution.is_some() {
                    TransformTemplateOutputFunctionKind::Color
                } else {
                    TransformTemplateOutputFunctionKind::Format
                },
                TransformTemplateOutputProducedKind::CemTree,
                writer_artifact
                    .identity
                    .color_profile
                    .clone()
                    .or_else(|| writer_artifact.identity.formatter_profile.clone())
                    .or_else(|| Some(formatter_profile.clone())),
            ),
            subject_type: "cem-tree".to_owned(),
            identity: writer_artifact.identity.clone(),
            options: TransformTemplateEncodeOptions::default(),
        },
        artifact: writer_artifact,
    };
    let writer_started = Instant::now();
    let mut composition = compose_transform_template_encoded_text_artifacts(
        &[evaluated],
        &writer_context,
        diagnostic_uri,
    );
    let writer_elapsed_ns = Some(writer_started.elapsed().as_nanos());
    if !composition.diagnostics.is_empty() {
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics: std::mem::take(&mut composition.diagnostics),
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    match composition.artifact {
        Some(mut artifact) => {
            if wrap_html_output {
                markdown_wrap_html_preview_artifact(&mut artifact, presentation_options.tab_size);
            }
            ConversionOutputPipelineExecution {
                output: Some(artifact.value),
                source_map: artifact.source_map,
                output_spans: artifact.output_spans,
                format_execution,
                color_execution,
                format_elapsed_ns,
                color_elapsed_ns,
                writer_elapsed_ns,
                formatted_cem_tree: Some(formatted_artifact),
                colored_cem_tree,
                diagnostics: Vec::new(),
            }
        }
        None => ConversionOutputPipelineExecution {
            output: None,
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree: Some(formatted_artifact),
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        },
    }
}

fn markdown_formatter_name_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let name = target_scope
        .cemt_formatter
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(MARKDOWN_FORMAT_CEMT_STAGE_SPEC.function_name);
    if name != MARKDOWN_FORMAT_CEMT_STAGE_SPEC.function_name {
        return Err(format!(
            "unsupported Markdown formatter `{name}`; first-class Markdown output currently supports `{}`",
            MARKDOWN_FORMAT_CEMT_STAGE_SPEC.function_name
        ));
    }
    Ok(name.to_owned())
}

fn markdown_formatter_profile_for_scope(target_scope: &ScopeConfig) -> Result<String, String> {
    let profile = target_scope
        .cemt_formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("compact");
    if !matches!(profile, "compact" | "pretty" | "tabular") {
        return Err(format!(
            "unsupported Markdown formatter profile `{profile}`; supported profiles are compact, pretty, and tabular"
        ));
    }
    Ok(profile.to_owned())
}

fn markdown_output_color_selection(
    output_color_type: Option<&str>,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    let Some(output_color_type) = output_color_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    parse_transform_template_output_color_type(output_color_type)
        .map(Some)
        .map_err(|message| {
            format!("invalid Markdown output color type `{output_color_type}`: {message}")
        })
}

fn markdown_output_color_selection_for_scope(
    target_scope: &ScopeConfig,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    markdown_output_color_selection(target_scope.output_color_type.as_deref())
}

fn markdown_output_color_selection_requests_color(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.output_color_type != "none"
}

fn markdown_output_color_selection_is_html(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "html-color"
        && markdown_output_color_selection_requests_color(selection)
}

fn markdown_output_color_selection_is_terminal(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "terminal-color"
        && markdown_output_color_selection_requests_color(selection)
}

fn markdown_cemt_color_profile_for_output(
    target_scope: &ScopeConfig,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> Result<Option<String>, String> {
    if let Some(name) = target_scope
        .cemt_colorizer
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if name != MARKDOWN_COLOR_CEMT_STAGE_SPEC.function_name {
            return Err(format!(
                "unsupported Markdown colorizer `{name}`; first-class Markdown output currently supports `{}`",
                MARKDOWN_COLOR_CEMT_STAGE_SPEC.function_name
            ));
        }
    }
    let explicit = target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty());
    let explicit = match explicit {
        Some("terminal" | "html" | "md") => explicit,
        Some("none") => None,
        Some(profile) => {
            return Err(format!(
                "unsupported Markdown color profile `{profile}`; supported profiles are terminal, html, md, and none"
            ));
        }
        None => None,
    };
    let inferred = output_color_selection
        .filter(|selection| markdown_output_color_selection_requests_color(selection))
        .and_then(|selection| match selection.target.category.as_str() {
            "html-color" => Some("html"),
            "terminal-color" => Some("terminal"),
            _ => None,
        });
    if let (Some(explicit), Some(inferred)) = (explicit, inferred) {
        if explicit != inferred {
            return Err(format!(
                "Markdown color profile `{explicit}` conflicts with output color type `{}`; use `{inferred}` or omit `--cemt-color-profile`",
                output_color_selection
                    .map(|selection| selection.output_color_type.as_str())
                    .unwrap_or_default()
            ));
        }
    }
    Ok(explicit
        .map(str::to_owned)
        .or_else(|| inferred.map(str::to_owned)))
}

const MARKDOWN_HTML_PREVIEW_CLASS: &str = "cem-output-markdown";

#[cfg(test)]
fn markdown_html_preview_prefix(tab_size: usize) -> String {
    crate::conversion_output::html_pre_container_prefix(MARKDOWN_HTML_PREVIEW_CLASS, tab_size)
}

fn markdown_wrap_html_preview_artifact(
    artifact: &mut TransformTemplateEncodedArtifact,
    tab_size: usize,
) {
    wrap_html_pre_container_artifact(artifact, MARKDOWN_HTML_PREVIEW_CLASS, tab_size);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarkdownFormatterPresentationOptions {
    line_ending: Option<FormatterLineEndingMode>,
    tab_size: usize,
}

impl MarkdownFormatterPresentationOptions {
    fn from_options(options: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut parsed = Self {
            line_ending: None,
            tab_size: default_formatter_tab_size(),
        };
        for (key, value) in options {
            match key.as_str() {
                "lineEnding" => {
                    parsed.line_ending = Some(parse_formatter_line_ending_option(key, value)?);
                }
                "tabSize" => {
                    parsed.tab_size = parse_positive_formatter_usize_option(key, value)?;
                }
                _ if key.starts_with("markdown.") => {
                    return Err(format!("unsupported Markdown formatter option `{key}`"));
                }
                _ => {}
            }
        }
        Ok(parsed)
    }
}

fn markdown_formatter_line_ending(
    source_line_ending: Option<&str>,
    options: &MarkdownFormatterPresentationOptions,
) -> Option<String> {
    resolve_formatter_line_ending(source_line_ending, options.line_ending)
}

fn markdown_output_function_descriptor(
    name: &str,
    category: &str,
    subject: &str,
    kind: TransformTemplateOutputFunctionKind,
    produces: TransformTemplateOutputProducedKind,
    profile: Option<String>,
) -> TransformTemplateOutputFunctionDescriptor {
    cemt_output_function_descriptor(CemtOutputFunctionDescriptorSpec {
        owner: "markdown",
        name,
        category,
        subject,
        kind,
        produces,
        content_type: MARKDOWN_CONTENT_TYPE,
        schema: MARKDOWN_SCHEMA_URI,
        canonical: false,
        profile,
    })
}

fn markdown_output_pipeline_failed(
    diagnostic_uri: Option<&str>,
    message: String,
) -> ConversionOutputPipelineExecution {
    markdown_output_pipeline_failed_with_timings(diagnostic_uri, message, None, None, None)
}

fn markdown_output_pipeline_failed_with_timings(
    diagnostic_uri: Option<&str>,
    message: String,
    format_elapsed_ns: Option<u128>,
    color_elapsed_ns: Option<u128>,
    writer_elapsed_ns: Option<u128>,
) -> ConversionOutputPipelineExecution {
    failed_pipeline_execution(
        "markdown-direct-output",
        Some("markdown"),
        diagnostic_uri,
        message,
        format_elapsed_ns,
        color_elapsed_ns,
        writer_elapsed_ns,
    )
}

fn conversion_output_pipeline_formatted_cem_tree_artifact(
    pipeline: &ConversionOutputPipeline,
    value: Value,
    source_map: Option<SourceMapStack>,
    output_spans: Vec<OutputSpan>,
) -> TransformTemplateEncodedArtifact {
    TransformTemplateEncodedArtifact {
        identity: TransformTemplateEncodedArtifactIdentity::from_options(
            TransformTemplateOutputProducedKind::CemTree,
            pipeline.cemt_target.clone(),
            &pipeline.cemt_options,
        ),
        value: conversion_output_pipeline_formatted_cem_tree_value(pipeline, value),
        source_map,
        output_spans,
        encoded: true,
    }
}

fn conversion_output_pipeline_formatted_cem_tree_value(
    pipeline: &ConversionOutputPipeline,
    value: Value,
) -> Value {
    let apply_envelope_defaults = |object: &mut serde_json::Map<String, Value>| {
        object
            .entry("contentType".to_owned())
            .or_insert_with(|| Value::String(pipeline.cemt_target.content_type.clone()));
        object
            .entry("schema".to_owned())
            .or_insert_with(|| Value::String(pipeline.cemt_target.schema.clone()));
        object
            .entry("category".to_owned())
            .or_insert_with(|| Value::String(pipeline.cemt_target.category.clone()));
        object
            .entry("mode".to_owned())
            .or_insert_with(|| Value::String(pipeline.cemt_options.mode.as_str().to_owned()));
        object
            .entry("canonical".to_owned())
            .or_insert_with(|| Value::Bool(pipeline.cemt_options.canonical));
        if let Some(formatter_profile) = pipeline.cemt_options.formatter_profile.as_ref() {
            object.insert(
                "formatterProfile".to_owned(),
                Value::String(formatter_profile.clone()),
            );
            object.entry("formatNodes".to_owned()).or_insert_with(|| {
                Value::Array(vec![
                    serde_json::json!({
                        "kind": "format-marker",
                        "name": "cem.format-tree",
                        "formatterRole": "formatter.boundary",
                        "formatterProfile": formatter_profile,
                    }),
                    serde_json::json!({
                        "kind": "format-decision",
                        "name": "converter-cemt",
                        "formatterRole": "formatter.converter",
                        "formatterProfile": formatter_profile,
                        "value": "converter CEMT produced formatted tree",
                    }),
                ])
            });
        }
    };

    match value {
        Value::Object(object) if object.get("kind").and_then(Value::as_str) == Some("cem-tree") => {
            Value::Object(object)
        }
        Value::Array(nodes) => {
            let mut object = serde_json::Map::new();
            object.insert("kind".to_owned(), Value::String("cem-tree".to_owned()));
            object.insert("nodes".to_owned(), Value::Array(nodes));
            apply_envelope_defaults(&mut object);
            conversion_output_pipeline_normalize_formatted_cem_tree_nodes(&mut object);
            Value::Object(object)
        }
        Value::Object(node) => {
            let mut object = serde_json::Map::new();
            object.insert("kind".to_owned(), Value::String("cem-tree".to_owned()));
            object.insert("node".to_owned(), Value::Object(node));
            apply_envelope_defaults(&mut object);
            conversion_output_pipeline_normalize_formatted_cem_tree_nodes(&mut object);
            Value::Object(object)
        }
        other => {
            let mut object = serde_json::Map::new();
            object.insert("kind".to_owned(), Value::String("cem-tree".to_owned()));
            object.insert("root".to_owned(), other);
            apply_envelope_defaults(&mut object);
            conversion_output_pipeline_normalize_formatted_cem_tree_nodes(&mut object);
            Value::Object(object)
        }
    }
}

fn conversion_output_pipeline_claimed_formatted_cem_tree_diagnostic(
    pipeline: &ConversionOutputPipeline,
    value: &Value,
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
) -> Option<Diagnostic> {
    let object = value.as_object()?;
    if object.get("kind").and_then(Value::as_str) != Some("cem-tree") {
        return None;
    }

    let formatter_profile = object
        .get("formatterProfile")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(formatter_profile) = formatter_profile else {
        return Some(output_pipeline_diagnostic(
            converter_id,
            diagnostic_node,
            diagnostic_uri,
            "converter output claims formatted CEM tree but omits required formatter metadata `formatterProfile`".to_owned(),
        ));
    };
    if let Some(expected) = pipeline
        .cemt_options
        .formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if formatter_profile != expected {
            return Some(output_pipeline_diagnostic(
                converter_id,
                diagnostic_node,
                diagnostic_uri,
                format!(
                    "converter output claims formatted CEM tree with formatterProfile `{formatter_profile}` but the output pipeline expects `{expected}`"
                ),
            ));
        }
    }

    let Some(format_nodes) = object
        .get("formatNodes")
        .and_then(Value::as_array)
        .filter(|nodes| !nodes.is_empty())
    else {
        return Some(output_pipeline_diagnostic(
            converter_id,
            diagnostic_node,
            diagnostic_uri,
            "converter output claims formatted CEM tree but omits required formatter metadata `formatNodes`".to_owned(),
        ));
    };
    let has_marker = format_nodes.iter().any(|node| {
        node.get("kind").and_then(Value::as_str) == Some("format-marker")
            && node.get("name").and_then(Value::as_str) == Some("cem.format-tree")
    });
    if !has_marker {
        return Some(output_pipeline_diagnostic(
            converter_id,
            diagnostic_node,
            diagnostic_uri,
            "converter output claims formatted CEM tree but `formatNodes` has no `cem.format-tree` formatter marker".to_owned(),
        ));
    }
    let has_decision = format_nodes.iter().any(|node| {
        node.get("kind").and_then(Value::as_str) == Some("format-decision")
            && node
                .get("formatterRole")
                .and_then(Value::as_str)
                .is_some_and(|role| role.starts_with("formatter."))
    });
    if !has_decision {
        return Some(output_pipeline_diagnostic(
            converter_id,
            diagnostic_node,
            diagnostic_uri,
            "converter output claims formatted CEM tree but `formatNodes` has no formatter decision node".to_owned(),
        ));
    }

    None
}

fn conversion_output_pipeline_normalize_formatted_cem_tree_nodes(
    object: &mut serde_json::Map<String, Value>,
) {
    for field in ["nodes", "node", "root"] {
        if let Some(value) = object.get_mut(field) {
            conversion_output_pipeline_normalize_formatted_cem_tree_node_value(value);
        }
    }
}

fn conversion_output_pipeline_normalize_formatted_cem_tree_node_value(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                conversion_output_pipeline_normalize_formatted_cem_tree_node_value(item);
            }
        }
        Value::Object(object) => {
            let kind = object
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if !kind.is_empty() {
                object.entry("sourceMap".to_owned()).or_insert(Value::Null);
            }
            if kind == "element" {
                object
                    .entry("attributes".to_owned())
                    .or_insert_with(|| Value::Array(Vec::new()));
                object
                    .entry("children".to_owned())
                    .or_insert_with(|| Value::Array(Vec::new()));
                object.entry("formatLayout".to_owned()).or_insert_with(|| {
                    serde_json::json!({
                        "kind": "format-decision",
                        "formatterRole": "formatter.layout",
                        "value": "inline",
                    })
                });
            }
            for field in ["attributes", "children", "nodes", "node", "root"] {
                if let Some(child) = object.get_mut(field) {
                    conversion_output_pipeline_normalize_formatted_cem_tree_node_value(child);
                }
            }
        }
        _ => {}
    }
}

fn execute_conversion_output_pipeline_from_formatted_artifact(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    pipeline: &ConversionOutputPipeline,
    formatted_artifact: TransformTemplateEncodedArtifact,
    format_execution: Option<ConversionOutputPipelineStageExecution>,
    format_elapsed_ns: Option<u128>,
    mut diagnostics: Vec<Diagnostic>,
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
) -> ConversionOutputPipelineExecution {
    let functions = conversion_cem_tree_output_function_registry(environment, pipeline);
    let implementations = TransformTemplateEncodeImplementationRegistry::with_builtin_encoders();
    let formatted_cem_tree = Some(formatted_artifact.clone());

    let (writer_artifact, writer_binding, color_execution, color_elapsed_ns, colored_cem_tree) =
        if conversion_output_pipeline_should_skip_color_stage(pipeline) {
            let formatter_profile = formatted_artifact
                .identity
                .formatter_profile
                .as_deref()
                .or(pipeline.cemt_options.formatter_profile.as_deref())
                .unwrap_or("compact");
            let writer_binding = TransformTemplateEncodeBinding {
                function: conversion_cem_tree_format_function_descriptor(formatter_profile),
                subject_type: "cem-tree".to_owned(),
                identity: formatted_artifact.identity.clone(),
                options: pipeline.cemt_options.clone(),
            };
            (formatted_artifact.clone(), writer_binding, None, None, None)
        } else {
            let color_request = TransformTemplateEncodeBindingRequest::new(
                formatted_artifact.value.clone(),
                pipeline.cemt_target.clone(),
            )
            .with_subject_type(conversion_output_pipeline_color_subject_type(
                formatted_artifact.identity.produces,
            ))
            .with_options(pipeline.cemt_options.clone());
            let color_binding = match functions
                .resolve_color_binding(&color_request, implementations.host_capabilities())
            {
                Ok(binding) => binding.into_encode_binding(),
                Err(message) => {
                    let mut diagnostic = message.diagnostic(diagnostic_uri);
                    diagnostic.node = diagnostic_node.map(str::to_owned);
                    diagnostics.push(diagnostic);
                    return ConversionOutputPipelineExecution {
                        output: None,
                        diagnostics,
                        format_execution,
                        format_elapsed_ns,
                        formatted_cem_tree: formatted_cem_tree.clone(),
                        ..ConversionOutputPipelineExecution::default()
                    };
                }
            };
            let color_started = Instant::now();
            let color_result = execute_conversion_cem_tree_color_stage(
                environment,
                &color_binding,
                &formatted_artifact.value,
            );
            let color_elapsed_ns = Some(color_started.elapsed().as_nanos());
            let (colored_output, color_execution) = match color_result {
                Ok(output) => output,
                Err(message) => {
                    diagnostics.push(output_pipeline_diagnostic(
                        converter_id,
                        diagnostic_node,
                        diagnostic_uri,
                        format!(
                            "CEMT colorizer `{}` failed: {message}",
                            color_binding.function.name
                        ),
                    ));
                    return ConversionOutputPipelineExecution {
                        output: None,
                        diagnostics,
                        format_execution,
                        format_elapsed_ns,
                        color_elapsed_ns,
                        formatted_cem_tree: formatted_cem_tree.clone(),
                        ..ConversionOutputPipelineExecution::default()
                    };
                }
            };
            let color_execution = Some(color_execution);
            let colored_artifact = color_binding.artifact_with_metadata(
                colored_output,
                formatted_artifact.source_map.clone(),
                formatted_artifact.output_spans.clone(),
            );
            if let Err(error) =
                colored_artifact.validate_insertion(&pipeline.cemt_insertion_context)
            {
                let mut diagnostic = error.diagnostic(diagnostic_uri);
                diagnostic.node = diagnostic_node.map(str::to_owned);
                diagnostics.push(diagnostic);
                return ConversionOutputPipelineExecution {
                    output: None,
                    diagnostics,
                    format_execution,
                    color_execution,
                    format_elapsed_ns,
                    color_elapsed_ns,
                    formatted_cem_tree,
                    ..ConversionOutputPipelineExecution::default()
                };
            }
            (
                colored_artifact.clone(),
                color_binding,
                color_execution,
                color_elapsed_ns,
                Some(colored_artifact),
            )
        };

    let evaluated = TransformTemplateEvaluatedEncodeExpression {
        expression: TransformTemplateEncodeExpression {
            owner: diagnostic_node.map(str::to_owned),
            expression: format!("{converter_id} output pipeline"),
            subject: "rendered-cem-tree".to_owned(),
            subject_type: Some("cem-tree".to_owned()),
            target: pipeline.cemt_target.clone(),
            options: pipeline.cemt_options.clone(),
        },
        subject: writer_artifact.value.clone(),
        binding: writer_binding,
        artifact: writer_artifact,
    };
    let writer_started = Instant::now();
    let composition = compose_transform_template_encoded_text_artifacts(
        &[evaluated],
        &pipeline.writer_insertion_context,
        diagnostic_uri,
    );
    let writer_elapsed_ns = Some(writer_started.elapsed().as_nanos());
    diagnostics.extend(composition.diagnostics);
    match composition.artifact {
        Some(artifact) => ConversionOutputPipelineExecution {
            output: Some(artifact.value),
            source_map: artifact.source_map,
            output_spans: artifact.output_spans,
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree,
            colored_cem_tree,
            diagnostics,
        },
        None => ConversionOutputPipelineExecution {
            output: None,
            diagnostics,
            format_execution,
            color_execution,
            format_elapsed_ns,
            color_elapsed_ns,
            writer_elapsed_ns,
            formatted_cem_tree,
            colored_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        },
    }
}

fn conversion_cem_tree_format_insertion_context(
    pipeline: &ConversionOutputPipeline,
) -> TransformTemplateEncodedArtifactInsertionContext {
    let mut context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &pipeline.cemt_target,
        Some(pipeline.cemt_produces),
    );
    context.formatter_profile = pipeline.cemt_options.formatter_profile.clone();
    context.mode = Some(pipeline.cemt_options.mode);
    context.canonical = Some(pipeline.cemt_options.canonical);
    context.source_map_policy = Some(pipeline.cemt_options.source_map_policy);
    context
}

fn conversion_output_pipeline_color_subject_type(
    produces: TransformTemplateOutputProducedKind,
) -> &'static str {
    match produces {
        TransformTemplateOutputProducedKind::Tokens => "tokens",
        TransformTemplateOutputProducedKind::CemTree => "cem-tree",
        TransformTemplateOutputProducedKind::Text => "string",
        TransformTemplateOutputProducedKind::Bytes => "bytes",
        TransformTemplateOutputProducedKind::Chunks => "chunks",
        TransformTemplateOutputProducedKind::Diagnostics => "diagnostics",
    }
}

fn conversion_output_pipeline_should_skip_color_stage(pipeline: &ConversionOutputPipeline) -> bool {
    let colorizer = pipeline
        .cemt_options
        .colorizer
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if colorizer.is_some() {
        return false;
    }

    let cemt_color_profile = pipeline
        .cemt_options
        .color_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let writer_color_profile = pipeline
        .writer_insertion_context
        .color_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    cemt_color_profile.is_none_or(|profile| profile == "none")
        && writer_color_profile.is_none_or(|profile| profile == "none")
}

fn conversion_cem_tree_output_function_registry(
    environment: &ConversionOutputPipelineEnvironment<'_>,
    pipeline: &ConversionOutputPipeline,
) -> TransformTemplateOutputFunctionRegistry {
    let mut registry = TransformTemplateOutputFunctionRegistry::new();
    registry.register(conversion_cem_tree_format_function_descriptor(
        pipeline
            .cemt_options
            .formatter_profile
            .as_deref()
            .unwrap_or("compact"),
    ));
    let mut profiles = BTreeSet::new();
    profiles.insert("classes".to_owned());
    profiles.insert("inline-style".to_owned());
    profiles.insert("none".to_owned());
    if let Some(profile) = pipeline.cemt_options.color_profile.as_deref() {
        profiles.insert(profile.to_owned());
    }
    for profile in profiles {
        registry.register(conversion_cem_tree_color_function_descriptor(&profile));
    }
    register_package_cem_tree_output_functions(&mut registry, environment, pipeline);
    registry
}

fn register_package_cem_tree_output_functions(
    registry: &mut TransformTemplateOutputFunctionRegistry,
    environment: &ConversionOutputPipelineEnvironment<'_>,
    pipeline: &ConversionOutputPipeline,
) {
    let Ok(package_id) = conversion_package_id_for_encoding_target(
        environment.schema_registry,
        &pipeline.cemt_target,
    ) else {
        return;
    };
    let mut registered = BTreeSet::new();

    for artifact in environment.conversion_registry.package_artifacts() {
        if !package_artifact_matches_cem_tree_target(artifact, &package_id, &pipeline.cemt_target) {
            continue;
        }

        let (expected_kind, canonical_name, requested_function_name, helper_artifact) =
            match artifact.kind.as_str() {
                "formatter" => (
                    TransformTemplateOutputFunctionKind::Format,
                    CEM_TREE_FORMAT_CEMT_STAGE_SPEC.function_name,
                    pipeline.cemt_options.formatter.as_deref(),
                    false,
                ),
                "colorizer" => (
                    TransformTemplateOutputFunctionKind::Color,
                    CEM_TREE_COLOR_CEMT_STAGE_SPEC.function_name,
                    pipeline.cemt_options.colorizer.as_deref(),
                    false,
                ),
                CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND => (
                    TransformTemplateOutputFunctionKind::Format,
                    CEM_TREE_FORMAT_CEMT_STAGE_SPEC.function_name,
                    None,
                    true,
                ),
                CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND => (
                    TransformTemplateOutputFunctionKind::Color,
                    CEM_TREE_COLOR_CEMT_STAGE_SPEC.function_name,
                    None,
                    true,
                ),
                _ => continue,
            };
        let Some(function_name) = artifact.function_name.as_deref() else {
            continue;
        };
        let requested_function_name = requested_function_name
            .map(str::trim)
            .filter(|name| !name.is_empty());
        if !helper_artifact {
            let Some(requested_function_name) = requested_function_name else {
                continue;
            };
            if function_name != requested_function_name {
                continue;
            }
            if function_name == canonical_name {
                continue;
            }
        }
        if helper_artifact
            && package_artifact_output_function_kind(&artifact.kind) != Some(expected_kind)
        {
            continue;
        }

        let Ok(module_options) =
            parse_cem_tree_cemt_output_artifact_module_options(environment, artifact)
        else {
            continue;
        };

        let Some(function) = module_options
            .output_functions
            .iter()
            .find(|function| {
                package_artifact_output_function_matches_profile(
                    function,
                    artifact,
                    expected_kind,
                    function_name,
                )
            })
            .or_else(|| {
                module_options.output_functions.iter().find(|function| {
                    function.kind == expected_kind
                        && function.name == function_name
                        && function.profile.is_none()
                })
            })
        else {
            continue;
        };
        let function = package_artifact_profiled_output_function(function, artifact);
        let key = (
            function.kind,
            function.name.clone(),
            function.profile.clone(),
        );
        if registered.insert(key) {
            registry.register(function);
        }
    }
}

fn package_artifact_output_function_matches_profile(
    function: &TransformTemplateOutputFunctionDescriptor,
    artifact: &ConversionPackageArtifactDescriptor,
    expected_kind: TransformTemplateOutputFunctionKind,
    function_name: &str,
) -> bool {
    function.kind == expected_kind
        && function.name == function_name
        && package_artifact_function_profile(artifact)
            .is_none_or(|profile| function.profile.as_deref() == Some(profile))
}

fn package_artifact_profiled_output_function(
    function: &TransformTemplateOutputFunctionDescriptor,
    artifact: &ConversionPackageArtifactDescriptor,
) -> TransformTemplateOutputFunctionDescriptor {
    let mut function = function.clone();
    if function.profile.is_none() {
        function.profile = package_artifact_function_profile(artifact).map(str::to_owned);
    }
    function
}

fn package_artifact_function_profile(
    artifact: &ConversionPackageArtifactDescriptor,
) -> Option<&str> {
    artifact
        .function_profile
        .as_deref()
        .or_else(|| artifact.formatter_profile.as_deref())
        .or_else(|| artifact.color_profile.as_deref())
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
}

fn conversion_cem_tree_format_function_descriptor(
    profile: &str,
) -> TransformTemplateOutputFunctionDescriptor {
    cemt_output_function_descriptor(CemtOutputFunctionDescriptorSpec {
        owner: "cem",
        name: "cem.format-tree",
        category: "cem-tree",
        subject: "cem-ast-node",
        kind: TransformTemplateOutputFunctionKind::Format,
        produces: TransformTemplateOutputProducedKind::CemTree,
        content_type: CEM_ML_CONTENT_TYPE,
        schema: CEM_ML_SCHEMA_URI,
        canonical: true,
        profile: Some(profile.to_owned()),
    })
}

fn conversion_cem_tree_color_function_descriptor(
    profile: &str,
) -> TransformTemplateOutputFunctionDescriptor {
    cemt_output_function_descriptor(CemtOutputFunctionDescriptorSpec {
        owner: "cem",
        name: "cem.color-tree",
        category: "cem-tree",
        subject: "cem-tree",
        kind: TransformTemplateOutputFunctionKind::Color,
        produces: TransformTemplateOutputProducedKind::CemTree,
        content_type: CEM_ML_CONTENT_TYPE,
        schema: CEM_ML_SCHEMA_URI,
        canonical: false,
        profile: Some(profile.to_owned()),
    })
}

fn conversion_template_identity(template: &ConversionTemplateDescriptor) -> FormatIdentity {
    FormatIdentity {
        content_type: Some(template.content_type.clone()),
        schema: template.schema.clone(),
        ..FormatIdentity::default()
    }
}

fn execute_rust_dom_projection_parity_fixture(
    descriptor: &ConversionDescriptor,
    fixture: &ConversionParityFixture,
) -> ConversionParityFixtureExecution {
    let Some(rust_symbol) = conversion_rust_or_fallback_symbol(descriptor) else {
        return conversion_parity_fixture_execution_error(
            descriptor,
            fixture,
            "converter has no Rust symbol or Rust fallback symbol".to_owned(),
        );
    };

    let result = match rust_symbol {
        "HtmlExportConverter" => conversion_dom_projection_fixture_string_output(
            fixture,
            ConversionDomProjectionOutput::Html,
        ),
        "XmlExportConverter" => {
            conversion_dom_projection_fixture_string_output(fixture, ConversionDomProjectionOutput::Xml)
        }
        "DomJsonDebugProjectionConverter" => {
            conversion_dom_projection_fixture_json_output(fixture)
        }
        _ => Err(format!(
            "Rust converter symbol `{rust_symbol}` is not supported by the DOM projection parity fixture executor"
        )),
    };

    match result {
        Ok(output) => ConversionParityFixtureExecution {
            output: Some(output),
            diagnostics: Vec::new(),
        },
        Err(message) => conversion_parity_fixture_execution_error(descriptor, fixture, message),
    }
}

fn conversion_rust_or_fallback_symbol(descriptor: &ConversionDescriptor) -> Option<&str> {
    descriptor.rust_symbol.as_deref().or_else(|| {
        descriptor
            .rust_fallback
            .as_ref()
            .map(|fallback| fallback.rust_symbol.as_str())
    })
}

fn conversion_parity_fixture_execution_error(
    descriptor: &ConversionDescriptor,
    fixture: &ConversionParityFixture,
    message: String,
) -> ConversionParityFixtureExecution {
    ConversionParityFixtureExecution {
        output: None,
        diagnostics: vec![Diagnostic {
            code: CONVERSION_PARITY_FIXTURE_EXECUTION_CODE.to_owned(),
            severity: Severity::Error,
            message: format!(
                "converter `{}` could not execute parity fixture `{}`: {}",
                descriptor.id, fixture.id, message
            ),
            node: Some(fixture.id.clone()),
            details: None,
            ..Diagnostic::default()
        }],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConversionDomProjectionOutput {
    Html,
    Xml,
}

fn conversion_dom_projection_fixture_string_output(
    fixture: &ConversionParityFixture,
    output: ConversionDomProjectionOutput,
) -> Result<Value, String> {
    let bytes = conversion_parity_fixture_input_bytes(fixture)?;
    let document = conversion_decode_dom_binary_projection(&bytes)?;
    let mut rendered = String::new();
    conversion_render_decoded_dom_children(
        &document,
        &document.root_children,
        output,
        &mut rendered,
    );
    Ok(Value::String(rendered))
}

fn conversion_dom_projection_fixture_json_output(
    fixture: &ConversionParityFixture,
) -> Result<Value, String> {
    let bytes = conversion_parity_fixture_input_bytes(fixture)?;
    let document = conversion_decode_dom_binary_projection(&bytes)?;
    Ok(conversion_decoded_dom_json(&document))
}

fn conversion_parity_fixture_input_bytes(
    fixture: &ConversionParityFixture,
) -> Result<Vec<u8>, String> {
    let input = fixture
        .input
        .as_object()
        .ok_or_else(|| "fixture input must be an object".to_owned())?;
    if let Some(content_type) = input.get("contentType").and_then(Value::as_str) {
        if content_type_essence(content_type) != CEM_DOM_PROJECTION_CONTENT_TYPE {
            return Err(format!(
                "fixture input content type `{content_type}` is not `{CEM_DOM_PROJECTION_CONTENT_TYPE}`"
            ));
        }
    }

    let bytes = input
        .get("bytes")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture input must contain a `bytes` array".to_owned())?;
    bytes
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|byte| u8::try_from(byte).ok())
                .ok_or_else(|| "fixture `bytes` array must contain byte values".to_owned())
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ConversionDecodedDomDocument {
    root_children: Vec<u32>,
    nodes: BTreeMap<u32, ConversionDecodedDomNode>,
}

#[derive(Debug, Clone)]
enum ConversionDecodedDomNode {
    Element {
        name: ConversionDecodedName,
        attributes: Vec<u32>,
        children: Vec<u32>,
    },
    Attribute {
        name: ConversionDecodedName,
        value: Option<String>,
    },
    Text(String),
    Whitespace(String),
    Comment(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
    Cdata(String),
    RawText(String),
    Error,
}

#[derive(Debug, Clone)]
struct ConversionDecodedName {
    namespace: String,
    local: String,
}

fn conversion_decode_dom_binary_projection(
    bytes: &[u8],
) -> Result<ConversionDecodedDomDocument, String> {
    let mut reader = ConversionBinaryReader::new(bytes);
    reader.read_magic()?;
    let version = reader.read_u16()?;
    if version != 1 {
        return Err(format!(
            "unsupported CEM binary projection version `{version}`"
        ));
    }
    let kind = reader.read_u8()?;
    if kind != 1 {
        return Err(format!(
            "expected DOM binary projection kind `1`, found `{kind}`"
        ));
    }
    let schema = reader.read_str()?;
    if schema != CEM_DOM_PROJECTION_SCHEMA_URI {
        return Err(format!(
            "expected DOM projection schema `{CEM_DOM_PROJECTION_SCHEMA_URI}`, found `{schema}`"
        ));
    }
    let content_type = reader.read_str()?;
    if content_type != CEM_DOM_PROJECTION_CONTENT_TYPE {
        return Err(format!(
            "expected DOM projection content type `{CEM_DOM_PROJECTION_CONTENT_TYPE}`, found `{content_type}`"
        ));
    }

    let node_count = reader.read_u32()?;
    let mut root_children = Vec::new();
    let mut nodes = BTreeMap::new();
    for _ in 0..node_count {
        match reader.read_node()? {
            ConversionDecodedBinaryNode::Document { children } => {
                root_children = children;
            }
            ConversionDecodedBinaryNode::Node { id, node } => {
                nodes.insert(id, node);
            }
        }
    }

    Ok(ConversionDecodedDomDocument {
        root_children,
        nodes,
    })
}

enum ConversionDecodedBinaryNode {
    Document {
        children: Vec<u32>,
    },
    Node {
        id: u32,
        node: ConversionDecodedDomNode,
    },
}

struct ConversionBinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ConversionBinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_magic(&mut self) -> Result<(), String> {
        let magic = self.read_exact(8)?;
        if magic != b"CEMPROJ\0" {
            return Err("fixture bytes do not start with CEM projection magic".to_owned());
        }
        Ok(())
    }

    fn read_node(&mut self) -> Result<ConversionDecodedBinaryNode, String> {
        let tag = self.read_u8()?;
        match tag {
            1 => {
                let _id = self.read_u32()?;
                self.skip_source_range()?;
                Ok(ConversionDecodedBinaryNode::Document {
                    children: self.read_id_list()?,
                })
            }
            2 => {
                let id = self.read_u32()?;
                self.skip_source_range()?;
                let name = self.read_expanded_name()?;
                let _has_explicit_boundary = self.read_bool()?;
                let attributes = self.read_id_list()?;
                let children = self.read_id_list()?;
                Ok(ConversionDecodedBinaryNode::Node {
                    id,
                    node: ConversionDecodedDomNode::Element {
                        name,
                        attributes,
                        children,
                    },
                })
            }
            3 => {
                let id = self.read_u32()?;
                self.skip_source_range()?;
                let name = self.read_expanded_name()?;
                let value = self.read_optional_str()?;
                Ok(ConversionDecodedBinaryNode::Node {
                    id,
                    node: ConversionDecodedDomNode::Attribute { name, value },
                })
            }
            4 => self.read_text_node(ConversionDecodedDomNode::Text),
            5 => self.read_text_node(ConversionDecodedDomNode::Whitespace),
            6 => self.read_text_node(ConversionDecodedDomNode::Comment),
            7 => {
                let id = self.read_u32()?;
                self.skip_source_range()?;
                let target = self.read_str()?;
                let data = self.read_str()?;
                Ok(ConversionDecodedBinaryNode::Node {
                    id,
                    node: ConversionDecodedDomNode::ProcessingInstruction { target, data },
                })
            }
            8 => self.read_text_node(ConversionDecodedDomNode::Cdata),
            9 => self.read_text_node(ConversionDecodedDomNode::RawText),
            10 => {
                let id = self.read_u32()?;
                self.skip_source_range()?;
                let _code = self.read_str()?;
                Ok(ConversionDecodedBinaryNode::Node {
                    id,
                    node: ConversionDecodedDomNode::Error,
                })
            }
            _ => Err(format!("unsupported DOM binary node tag `{tag}`")),
        }
    }

    fn read_text_node(
        &mut self,
        build: impl FnOnce(String) -> ConversionDecodedDomNode,
    ) -> Result<ConversionDecodedBinaryNode, String> {
        let id = self.read_u32()?;
        self.skip_source_range()?;
        let data = self.read_str()?;
        Ok(ConversionDecodedBinaryNode::Node {
            id,
            node: build(data),
        })
    }

    fn read_expanded_name(&mut self) -> Result<ConversionDecodedName, String> {
        let namespace = self.read_str()?;
        let local = self.read_str()?;
        if self.read_bool()? {
            let _schema_id = self.read_u32()?;
        }
        Ok(ConversionDecodedName { namespace, local })
    }

    fn read_id_list(&mut self) -> Result<Vec<u32>, String> {
        let len = self.read_u32()? as usize;
        (0..len).map(|_| self.read_u32()).collect()
    }

    fn skip_source_range(&mut self) -> Result<(), String> {
        if self.read_bool()? {
            let _start = self.read_u64()?;
            let _len = self.read_u32()?;
        }
        Ok(())
    }

    fn read_optional_str(&mut self) -> Result<Option<String>, String> {
        if self.read_bool()? {
            self.read_str().map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_str(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|err| err.to_string())
    }

    fn read_bool(&mut self) -> Result<bool, String> {
        Ok(self.read_u8()? != 0)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "fixture offset overflow".to_owned())?;
        if end > self.bytes.len() {
            return Err(
                "fixture bytes ended before the DOM projection record completed".to_owned(),
            );
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}

fn conversion_render_decoded_dom_children(
    document: &ConversionDecodedDomDocument,
    children: &[u32],
    output: ConversionDomProjectionOutput,
    out: &mut String,
) {
    for child in children {
        conversion_render_decoded_dom_node(document, *child, output, out);
    }
}

fn conversion_render_decoded_dom_node(
    document: &ConversionDecodedDomDocument,
    node_id: u32,
    output: ConversionDomProjectionOutput,
    out: &mut String,
) {
    let Some(node) = document.nodes.get(&node_id) else {
        return;
    };
    match node {
        ConversionDecodedDomNode::Element {
            name,
            attributes,
            children,
        } => {
            if name.local.starts_with('@') {
                return;
            }
            out.push('<');
            conversion_push_decoded_name(out, name);
            let mut sorted_attributes = attributes.clone();
            sorted_attributes.sort_by(|a, b| {
                conversion_decoded_attribute_name(document, *a)
                    .cmp(&conversion_decoded_attribute_name(document, *b))
            });
            for attribute in sorted_attributes {
                conversion_render_decoded_dom_attribute(document, attribute, output, out);
            }
            out.push('>');
            conversion_render_decoded_dom_children(document, children, output, out);
            out.push_str("</");
            conversion_push_decoded_name(out, name);
            out.push('>');
        }
        ConversionDecodedDomNode::Text(data) => conversion_escape_text_into(out, data),
        ConversionDecodedDomNode::Whitespace(data) => out.push_str(data),
        ConversionDecodedDomNode::Comment(data) => {
            out.push_str("<!--");
            out.push_str(data);
            out.push_str("-->");
        }
        ConversionDecodedDomNode::ProcessingInstruction { target, data } => {
            out.push_str("<?");
            out.push_str(target);
            if !data.is_empty() {
                out.push(' ');
                out.push_str(data);
            }
            out.push_str("?>");
        }
        ConversionDecodedDomNode::Cdata(data) => {
            out.push_str("<![CDATA[");
            out.push_str(data);
            out.push_str("]]>");
        }
        ConversionDecodedDomNode::RawText(data) => out.push_str(data),
        ConversionDecodedDomNode::Attribute { .. } | ConversionDecodedDomNode::Error => {}
    }
}

fn conversion_render_decoded_dom_attribute(
    document: &ConversionDecodedDomDocument,
    node_id: u32,
    output: ConversionDomProjectionOutput,
    out: &mut String,
) {
    let Some(ConversionDecodedDomNode::Attribute { name, value }) = document.nodes.get(&node_id)
    else {
        return;
    };
    out.push(' ');
    conversion_push_decoded_name(out, name);
    match (output, value) {
        (ConversionDomProjectionOutput::Html, None) => {}
        (_, value) => {
            out.push_str("=\"");
            if let Some(value) = value {
                match output {
                    ConversionDomProjectionOutput::Html => {
                        conversion_escape_html_attribute_into(out, value);
                    }
                    ConversionDomProjectionOutput::Xml => {
                        conversion_escape_xml_attribute_into(out, value);
                    }
                }
            }
            out.push('"');
        }
    }
}

fn conversion_decoded_attribute_name(
    document: &ConversionDecodedDomDocument,
    node_id: u32,
) -> (String, String) {
    match document.nodes.get(&node_id) {
        Some(ConversionDecodedDomNode::Attribute { name, .. }) => {
            (name.namespace.clone(), name.local.clone())
        }
        _ => (String::new(), String::new()),
    }
}

fn conversion_push_decoded_name(out: &mut String, name: &ConversionDecodedName) {
    if !name.namespace.is_empty() {
        out.push_str(&name.namespace);
        out.push(':');
    }
    out.push_str(&name.local);
}

fn conversion_escape_text_into(out: &mut String, data: &str) {
    for c in data.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn conversion_escape_html_attribute_into(out: &mut String, data: &str) {
    for c in data.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            _ => out.push(c),
        }
    }
}

fn conversion_escape_xml_attribute_into(out: &mut String, data: &str) {
    for c in data.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn conversion_decoded_dom_json(document: &ConversionDecodedDomDocument) -> Value {
    let children = document
        .root_children
        .iter()
        .filter_map(|node_id| conversion_decoded_dom_node_json(document, *node_id))
        .collect::<Vec<_>>();
    serde_json::json!({
        "kind": "document",
        "children": children,
    })
}

fn conversion_decoded_dom_node_json(
    document: &ConversionDecodedDomDocument,
    node_id: u32,
) -> Option<Value> {
    let node = document.nodes.get(&node_id)?;
    Some(match node {
        ConversionDecodedDomNode::Element {
            name,
            attributes,
            children,
        } => {
            let attributes = attributes
                .iter()
                .filter_map(|attribute_id| match document.nodes.get(attribute_id) {
                    Some(ConversionDecodedDomNode::Attribute { name, value }) => {
                        Some(serde_json::json!({
                            "name": name.local,
                            "namespace": name.namespace,
                            "value": value,
                        }))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let children = children
                .iter()
                .filter_map(|child_id| conversion_decoded_dom_node_json(document, *child_id))
                .collect::<Vec<_>>();
            serde_json::json!({
                "kind": "element",
                "name": name.local,
                "namespace": name.namespace,
                "attributes": attributes,
                "children": children,
                "byteRange": Value::Null,
            })
        }
        ConversionDecodedDomNode::Attribute { .. } => return None,
        ConversionDecodedDomNode::Text(data) => serde_json::json!({
            "kind": "text",
            "data": data,
            "byteRange": Value::Null,
        }),
        ConversionDecodedDomNode::Whitespace(data) => serde_json::json!({
            "kind": "whitespace",
            "data": data,
            "byteRange": Value::Null,
        }),
        ConversionDecodedDomNode::Comment(data) => serde_json::json!({
            "kind": "comment",
            "data": data,
            "byteRange": Value::Null,
        }),
        ConversionDecodedDomNode::ProcessingInstruction { target, data } => serde_json::json!({
            "kind": "processing-instruction",
            "name": target,
            "target": target,
            "data": data,
            "byteRange": Value::Null,
        }),
        ConversionDecodedDomNode::Cdata(data) => serde_json::json!({
            "kind": "cdata",
            "data": data,
            "byteRange": Value::Null,
        }),
        ConversionDecodedDomNode::RawText(data) => serde_json::json!({
            "kind": "raw-text",
            "data": data,
            "byteRange": Value::Null,
        }),
        ConversionDecodedDomNode::Error => serde_json::json!({
            "kind": "error",
            "byteRange": Value::Null,
        }),
    })
}

pub fn evaluate_conversion_parity_fixtures(
    contract: &ConversionParityContract<'_>,
    fixtures: &[ConversionParityFixture],
    executor: &dyn ConversionParityFixtureExecutor,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for fixture in fixtures {
        let cemt_execution = executor.execute_conversion_parity_fixture(contract.cemt, fixture);
        let native_execution = executor.execute_conversion_parity_fixture(contract.native, fixture);

        if !conversion_diagnostics_equivalent(
            &cemt_execution.diagnostics,
            &native_execution.diagnostics,
        ) {
            diagnostics.push(conversion_parity_fixture_diagnostic(
                contract,
                fixture,
                "diagnostics",
                "CEMT and native diagnostic streams differ".to_owned(),
            ));
        }

        if !conversion_fixture_expected_diagnostics_equivalent(&cemt_execution.diagnostics, fixture)
        {
            diagnostics.push(conversion_parity_fixture_diagnostic(
                contract,
                fixture,
                "cemt.diagnostics",
                "CEMT diagnostics differ from fixture expectations".to_owned(),
            ));
        }

        if !conversion_fixture_expected_diagnostics_equivalent(
            &native_execution.diagnostics,
            fixture,
        ) {
            diagnostics.push(conversion_parity_fixture_diagnostic(
                contract,
                fixture,
                "native.diagnostics",
                "native diagnostics differ from fixture expectations".to_owned(),
            ));
        }

        match (&cemt_execution.output, &native_execution.output) {
            (Some(cemt_output), Some(native_output)) => {
                if compare_conversion_parity_outputs(contract, cemt_output, native_output).is_some()
                {
                    diagnostics.push(conversion_parity_fixture_diagnostic(
                        contract,
                        fixture,
                        "output",
                        "CEMT and native outputs differ".to_owned(),
                    ));
                }
            }
            (None, None) if conversion_parity_fixture_allows_missing_output(fixture) => {}
            (None, None) => diagnostics.push(conversion_parity_fixture_diagnostic(
                contract,
                fixture,
                "output",
                "both converters completed without an output value".to_owned(),
            )),
            (None, Some(_)) => diagnostics.push(conversion_parity_fixture_diagnostic(
                contract,
                fixture,
                "output",
                "CEMT converter completed without an output value".to_owned(),
            )),
            (Some(_), None) => diagnostics.push(conversion_parity_fixture_diagnostic(
                contract,
                fixture,
                "output",
                "native converter completed without an output value".to_owned(),
            )),
        }
    }

    diagnostics
}

pub fn evaluate_declared_conversion_parity_contracts(
    registry: &ConversionRegistry,
    package_root: impl AsRef<Path>,
    executor: &dyn ConversionParityFixtureExecutor,
) -> Vec<Diagnostic> {
    let package_root = package_root.as_ref();
    let (contracts, mut diagnostics) = registry.cemt_native_parity_contracts();

    for contract in contracts {
        match load_conversion_parity_fixtures(contract.cemt, package_root) {
            Ok(fixtures) => {
                diagnostics.extend(evaluate_conversion_parity_fixtures(
                    &contract, &fixtures, executor,
                ));
            }
            Err(error) => diagnostics.push(conversion_parity_fixture_load_diagnostic(error)),
        }
    }

    diagnostics
}

fn conversion_parity_outputs_match(
    contract: &ConversionParityContract<'_>,
    cemt_output: &Value,
    native_output: &Value,
) -> bool {
    match contract.mode {
        ConversionParityMode::ParseEquivalent => {
            conversion_parse_equivalent_outputs_match(contract, cemt_output, native_output)
        }
        ConversionParityMode::TokenEquivalent => {
            conversion_token_equivalent_outputs_match(contract, cemt_output, native_output)
        }
        ConversionParityMode::DiagnosticEquivalent => {
            conversion_diagnostic_equivalent_outputs_match(cemt_output, native_output)
        }
        ConversionParityMode::ByteExact => cemt_output == native_output,
    }
}

fn conversion_diagnostic_equivalent_outputs_match(
    cemt_output: &Value,
    native_output: &Value,
) -> bool {
    match (
        conversion_diagnostic_projection(cemt_output),
        conversion_diagnostic_projection(native_output),
    ) {
        (Some(mut cemt), Some(mut native)) => {
            cemt.sort_by_key(conversion_diagnostic_projection_sort_key);
            native.sort_by_key(conversion_diagnostic_projection_sort_key);
            cemt == native
        }
        _ => cemt_output == native_output,
    }
}

fn conversion_diagnostics_equivalent(cemt: &[Diagnostic], native: &[Diagnostic]) -> bool {
    let (Ok(cemt), Ok(native)) = (serde_json::to_value(cemt), serde_json::to_value(native)) else {
        return false;
    };
    conversion_diagnostic_equivalent_outputs_match(&cemt, &native)
}

fn conversion_fixture_expected_diagnostics_equivalent(
    diagnostics: &[Diagnostic],
    fixture: &ConversionParityFixture,
) -> bool {
    if !fixture.expected_diagnostic_codes.is_empty() {
        let mut expected = fixture.expected_diagnostic_codes.clone();
        expected.sort();
        return conversion_diagnostic_codes(diagnostics) == expected;
    }
    conversion_diagnostics_equivalent(diagnostics, &fixture.expected_diagnostics)
}

fn conversion_diagnostic_codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut codes = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect::<Vec<_>>();
    codes.sort();
    codes
}

fn conversion_parity_fixture_allows_missing_output(fixture: &ConversionParityFixture) -> bool {
    !fixture.expected_diagnostic_codes.is_empty()
        || fixture
            .expected_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
}

fn conversion_parity_fixture_path(package_root: &Path, fixture_path: &str) -> PathBuf {
    let fixture_path = Path::new(fixture_path);
    if fixture_path.is_absolute() {
        fixture_path.to_path_buf()
    } else {
        package_root.join(fixture_path)
    }
}

fn conversion_parity_fixture_input_value(
    descriptor: &ConversionParityFixtureDescriptor,
    bytes: Vec<u8>,
) -> Value {
    let mut input = serde_json::Map::new();
    input.insert("path".to_owned(), Value::String(descriptor.path.clone()));
    if let Some(content_type) = descriptor.content_type.as_ref() {
        input.insert(
            "contentType".to_owned(),
            Value::String(content_type.clone()),
        );
    }
    if let Some(schema) = descriptor.schema.as_ref() {
        input.insert("schema".to_owned(), Value::String(schema.clone()));
    }
    input.insert(
        "bytes".to_owned(),
        Value::Array(
            bytes
                .into_iter()
                .map(|byte| Value::from(u64::from(byte)))
                .collect(),
        ),
    );
    Value::Object(input)
}

fn conversion_parity_fixture_diagnostic(
    contract: &ConversionParityContract<'_>,
    fixture: &ConversionParityFixture,
    field: &str,
    detail: String,
) -> Diagnostic {
    let mut diagnostic = conversion_parity_diagnostic(
        CONVERSION_PARITY_DRIFT_CODE,
        format!(
            "CEMT converter `{}` and native converter `{}` produced fixture `{}` drift at `{}` under `{}` parity: {}",
            contract.cemt.id,
            contract.native.id,
            fixture.id,
            field,
            conversion_parity_mode_selector(contract.mode),
            detail
        ),
    );
    diagnostic.node = Some(fixture.id.clone());
    diagnostic
}

fn conversion_parity_fixture_load_diagnostic(
    error: ConversionParityFixtureLoadError,
) -> Diagnostic {
    let node = match &error {
        ConversionParityFixtureLoadError::Read { fixture_id, .. } => Some(fixture_id.clone()),
    };
    Diagnostic {
        code: CONVERSION_PARITY_FIXTURE_LOAD_CODE.to_owned(),
        severity: Severity::Error,
        message: error.to_string(),
        node,
        details: None,
        ..Diagnostic::default()
    }
}

fn conversion_token_equivalent_outputs_match(
    contract: &ConversionParityContract<'_>,
    cemt_output: &Value,
    native_output: &Value,
) -> bool {
    let Some(output_syntax) = contract.cemt.output_contract.output_syntax else {
        return cemt_output == native_output;
    };
    let (Some(cemt_output), Some(native_output)) = (cemt_output.as_str(), native_output.as_str())
    else {
        return cemt_output == native_output;
    };

    match (
        conversion_token_projection(output_syntax, cemt_output),
        conversion_token_projection(output_syntax, native_output),
    ) {
        (Some(cemt), Some(native)) => cemt == native,
        _ => cemt_output == native_output,
    }
}

fn conversion_parse_equivalent_outputs_match(
    contract: &ConversionParityContract<'_>,
    cemt_output: &Value,
    native_output: &Value,
) -> bool {
    let Some(output_syntax) = contract.cemt.output_contract.output_syntax else {
        return cemt_output == native_output;
    };
    let (Some(cemt_output), Some(native_output)) = (cemt_output.as_str(), native_output.as_str())
    else {
        return cemt_output == native_output;
    };

    match (
        conversion_parse_projection(output_syntax, cemt_output),
        conversion_parse_projection(output_syntax, native_output),
    ) {
        (Some(cemt), Some(native)) => cemt == native,
        _ => cemt_output == native_output,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConversionDiagnosticProjectionEvent {
    code: String,
    severity: String,
    uri: Option<String>,
    line: Option<u64>,
    column: Option<u64>,
    byte_offset: Option<u64>,
    node: Option<String>,
    source_map: Option<String>,
}

fn conversion_diagnostic_projection(
    output: &Value,
) -> Option<Vec<ConversionDiagnosticProjectionEvent>> {
    let diagnostics = match output {
        Value::Array(diagnostics) => diagnostics.as_slice(),
        Value::Object(object) if object.contains_key("code") && object.contains_key("severity") => {
            std::slice::from_ref(output)
        }
        Value::Object(object) => object.get("diagnostics")?.as_array()?.as_slice(),
        _ => return None,
    };

    diagnostics
        .iter()
        .map(conversion_diagnostic_projection_event)
        .collect()
}

fn conversion_diagnostic_projection_event(
    diagnostic: &Value,
) -> Option<ConversionDiagnosticProjectionEvent> {
    let object = diagnostic.as_object()?;
    let code = conversion_required_string_field(object, "code")?;
    let severity = conversion_required_string_field(object, "severity")?.to_ascii_lowercase();
    let uri = conversion_optional_string_field(object, "uri")?;
    let line = conversion_optional_u64_field(object, "line")?;
    let column = conversion_optional_u64_field(object, "column")?;
    let byte_offset = conversion_optional_u64_field(object, "byteOffset")?;
    let node = conversion_optional_string_field(object, "node")?;
    let source_map = match object.get("sourceMap") {
        Some(Value::Null) | None => None,
        Some(source_map) => Some(conversion_canonical_json_string(source_map)?),
    };

    Some(ConversionDiagnosticProjectionEvent {
        code,
        severity,
        uri,
        line,
        column,
        byte_offset,
        node,
        source_map,
    })
}

fn conversion_required_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Option<String> {
    match object.get(field) {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn conversion_optional_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Option<Option<String>> {
    match object.get(field) {
        Some(Value::String(value)) => Some(Some(value.clone())),
        Some(Value::Null) | None => Some(None),
        _ => None,
    }
}

fn conversion_optional_u64_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Option<Option<u64>> {
    match object.get(field) {
        Some(Value::Number(value)) => value.as_u64().map(Some),
        Some(Value::Null) | None => Some(None),
        _ => None,
    }
}

fn conversion_canonical_json_string(value: &Value) -> Option<String> {
    serde_json::to_string(&conversion_canonical_json_value(value)).ok()
}

fn conversion_canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.iter().map(conversion_canonical_json_value).collect())
        }
        Value::Object(object) => {
            let mut canonical = serde_json::Map::new();
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            for key in keys {
                if let Some(value) = object.get(key) {
                    canonical.insert(key.clone(), conversion_canonical_json_value(value));
                }
            }
            Value::Object(canonical)
        }
        _ => value.clone(),
    }
}

fn conversion_diagnostic_projection_sort_key(
    diagnostic: &ConversionDiagnosticProjectionEvent,
) -> String {
    format!("{diagnostic:?}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConversionTokenProjectionEvent {
    NodeStart { name: String },
    NodeEnd { name: Option<String> },
    Attribute { name: String, value: Option<String> },
    Text(String),
    Comment(String),
    ProcessingInstruction { target: String, data: String },
    ExpressionNode(String),
    AnonymousScopeStart,
    Directive { name: String, data: String },
    RichContent(String),
    Error(String),
}

fn conversion_token_projection(
    output_syntax: ConversionOutputSyntax,
    output: &str,
) -> Option<Vec<ConversionTokenProjectionEvent>> {
    let source = BytesSource::new(SourceId(1), output.as_bytes().to_vec());
    match output_syntax {
        ConversionOutputSyntax::Html => Some(conversion_collect_token_projection(
            HtmlTokenizer::from_source(source),
        )),
        ConversionOutputSyntax::Xml => Some(conversion_collect_token_projection(
            XmlTokenizer::from_source(source),
        )),
        _ => None,
    }
}

fn conversion_collect_token_projection(
    mut tokenizer: impl SchemaTokenizer,
) -> Vec<ConversionTokenProjectionEvent> {
    let mut projection = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        match token.kind {
            SchemaTokenKind::NodeStart { name } => {
                projection.push(ConversionTokenProjectionEvent::NodeStart { name });
            }
            SchemaTokenKind::NodeEnd { name } => {
                projection.push(ConversionTokenProjectionEvent::NodeEnd { name });
            }
            SchemaTokenKind::Attribute { name, value, .. } => {
                projection.push(ConversionTokenProjectionEvent::Attribute { name, value });
            }
            SchemaTokenKind::Text(data) => {
                projection.push(ConversionTokenProjectionEvent::Text(data));
            }
            SchemaTokenKind::Trivia(_) => {}
            SchemaTokenKind::Comment(data) => {
                projection.push(ConversionTokenProjectionEvent::Comment(data));
            }
            SchemaTokenKind::ProcessingInstruction { target, data } => {
                projection
                    .push(ConversionTokenProjectionEvent::ProcessingInstruction { target, data });
            }
            SchemaTokenKind::ExpressionNode(data) => {
                projection.push(ConversionTokenProjectionEvent::ExpressionNode(data));
            }
            SchemaTokenKind::AnonymousScopeStart => {
                projection.push(ConversionTokenProjectionEvent::AnonymousScopeStart);
            }
            SchemaTokenKind::Directive { name, data } => {
                projection.push(ConversionTokenProjectionEvent::Directive { name, data });
            }
            SchemaTokenKind::RichContent { data } => {
                projection.push(ConversionTokenProjectionEvent::RichContent(data));
            }
            SchemaTokenKind::Error { code } => {
                projection.push(ConversionTokenProjectionEvent::Error(code));
            }
        }
    }
    projection
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConversionParseProjectionEvent {
    Open {
        name: String,
        attributes: Vec<(String, Option<String>)>,
    },
    Close(String),
    Text(String),
    Comment(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
}

fn conversion_parse_projection(
    output_syntax: ConversionOutputSyntax,
    output: &str,
) -> Option<Vec<ConversionParseProjectionEvent>> {
    let source = BytesSource::new(SourceId(1), output.as_bytes().to_vec());
    let document = match output_syntax {
        ConversionOutputSyntax::Html => {
            CemAstBuilder::new(CemEventNormalizer::new(HtmlTokenizer::from_source(source))).build()
        }
        ConversionOutputSyntax::Xml => {
            CemAstBuilder::new(CemEventNormalizer::new(XmlTokenizer::from_source(source))).build()
        }
        _ => return None,
    };
    if document
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.severity, Severity::Error | Severity::Fatal))
    {
        return None;
    }

    let mut projection = Vec::new();
    if let Some(CemAstNode::Document { root_children, .. }) = document.root() {
        for child in root_children {
            append_conversion_parse_projection_event(&document, *child, &mut projection);
        }
    }
    Some(projection)
}

fn append_conversion_parse_projection_event(
    document: &CemDocument,
    node_id: AstNodeId,
    projection: &mut Vec<ConversionParseProjectionEvent>,
) {
    let Some(node) = document.get(node_id) else {
        return;
    };
    match node {
        CemAstNode::Element {
            expanded_name,
            attributes,
            children,
            ..
        } => {
            let name = conversion_expanded_name_selector(expanded_name);
            let mut projected_attributes = attributes
                .iter()
                .filter_map(|attribute_id| match document.get(*attribute_id) {
                    Some(CemAstNode::Attribute {
                        expanded_name,
                        value,
                        ..
                    }) => Some((
                        conversion_expanded_name_selector(expanded_name),
                        value
                            .as_deref()
                            .map(conversion_normalize_character_references),
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if conversion_is_cem_color_wrapper_element(&name, &projected_attributes) {
                for child in children {
                    append_conversion_parse_projection_event(document, *child, projection);
                }
                return;
            }
            projected_attributes =
                conversion_normalize_parse_projection_attributes(projected_attributes);
            projected_attributes.sort();
            projection.push(ConversionParseProjectionEvent::Open {
                name: name.clone(),
                attributes: projected_attributes,
            });
            for child in children {
                append_conversion_parse_projection_event(document, *child, projection);
            }
            projection.push(ConversionParseProjectionEvent::Close(name));
        }
        CemAstNode::Text { data, .. }
        | CemAstNode::Cdata { data, .. }
        | CemAstNode::RawText { data, .. } => {
            let data = conversion_normalize_character_references(data);
            if !data.trim().is_empty() {
                projection.push(ConversionParseProjectionEvent::Text(data));
            }
        }
        CemAstNode::Comment { data, .. } => {
            projection.push(ConversionParseProjectionEvent::Comment(data.clone()));
        }
        CemAstNode::ProcessingInstruction { target, data, .. } => {
            projection.push(ConversionParseProjectionEvent::ProcessingInstruction {
                target: target.clone(),
                data: data.clone(),
            });
        }
        CemAstNode::Document { .. }
        | CemAstNode::Whitespace { .. }
        | CemAstNode::Attribute { .. }
        | CemAstNode::Error { .. } => {}
    }
}

fn conversion_normalize_parse_projection_attributes(
    attributes: Vec<(String, Option<String>)>,
) -> Vec<(String, Option<String>)> {
    let has_cem_color_class = attributes.iter().any(|(name, value)| {
        name == "class"
            && value
                .as_deref()
                .is_some_and(conversion_class_contains_cem_color_marker)
    });

    attributes
        .into_iter()
        .filter_map(|(name, value)| match name.as_str() {
            "class" => {
                conversion_normalize_parse_projection_class(value).map(|value| (name, Some(value)))
            }
            "data-role"
                if has_cem_color_class
                    && value
                        .as_deref()
                        .is_some_and(conversion_is_cem_color_role_value) =>
            {
                None
            }
            "data-cem-attribute-roles" if has_cem_color_class => None,
            "style"
                if has_cem_color_class
                    && value.as_deref().is_some_and(|value| {
                        value.contains("--cem-color-") || value.contains("cem-color-")
                    }) =>
            {
                None
            }
            _ => Some((name, value)),
        })
        .collect()
}

fn conversion_is_cem_color_wrapper_element(
    name: &str,
    attributes: &[(String, Option<String>)],
) -> bool {
    name == "span"
        && attributes.iter().any(|(name, value)| {
            name == "class"
                && value
                    .as_deref()
                    .is_some_and(conversion_class_contains_cem_color_marker)
        })
        && attributes.iter().any(|(name, value)| {
            name == "data-role"
                && value
                    .as_deref()
                    .is_some_and(conversion_is_cem_color_role_value)
        })
}

fn conversion_class_contains_cem_color_marker(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|token| token == "cem-color" || token.starts_with("cem-color-"))
}

fn conversion_normalize_parse_projection_class(value: Option<String>) -> Option<String> {
    let value = value?;
    let class = value
        .split_whitespace()
        .filter(|token| *token != "cem-color" && !token.starts_with("cem-color-"))
        .collect::<Vec<_>>()
        .join(" ");
    (!class.is_empty()).then_some(class)
}

fn conversion_is_cem_color_role_value(value: &str) -> bool {
    matches!(
        value.split_once('.').map(|(family, _)| family),
        Some("diagnostic" | "diff" | "source" | "status" | "syntax")
    )
}

fn conversion_normalize_character_references(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(ampersand) = rest.find('&') {
        output.push_str(&rest[..ampersand]);
        let after_ampersand = &rest[ampersand + '&'.len_utf8()..];
        if let Some(semicolon) = after_ampersand.find(';') {
            let reference = &after_ampersand[..semicolon];
            if let Some(ch) = conversion_character_reference(reference) {
                output.push(ch);
                rest = &after_ampersand[semicolon + ';'.len_utf8()..];
                continue;
            }
        }
        output.push('&');
        rest = after_ampersand;
    }

    output.push_str(rest);
    output
}

fn conversion_character_reference(reference: &str) -> Option<char> {
    match reference {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => {
            let codepoint = reference
                .strip_prefix("#x")
                .or_else(|| reference.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| {
                    reference
                        .strip_prefix('#')
                        .and_then(|decimal| decimal.parse::<u32>().ok())
                })?;
            char::from_u32(codepoint)
        }
    }
}

fn conversion_expanded_name_selector(expanded_name: &crate::parser::ExpandedName) -> String {
    if expanded_name.namespace_uri.is_empty() {
        expanded_name.local_name.clone()
    } else {
        format!(
            "{}:{}",
            expanded_name.namespace_uri, expanded_name.local_name
        )
    }
}

fn conversion_parity_mode_selector(mode: ConversionParityMode) -> &'static str {
    match mode {
        ConversionParityMode::ByteExact => "byte-exact",
        ConversionParityMode::TokenEquivalent => "token-equivalent",
        ConversionParityMode::ParseEquivalent => "parse-equivalent",
        ConversionParityMode::DiagnosticEquivalent => "diagnostic-equivalent",
    }
}

fn conversion_parity_diagnostic(code: &'static str, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        ..Diagnostic::default()
    }
}

fn conversion_output_safety_options(
    output_contract: &ConversionOutputContractDescriptor,
    category: &str,
) -> TransformTemplateEncodeOptions {
    TransformTemplateEncodeOptions {
        canonical: output_contract
            .formatter_profile
            .as_deref()
            .is_some_and(|profile| matches!(profile, "compact" | "canonical")),
        formatter_profile: output_contract.formatter_profile.clone(),
        color_profile: output_contract.color_profile.clone(),
        mode: conversion_output_artifact_mode(category),
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    }
}

fn conversion_output_artifact_mode(category: &str) -> TransformTemplateEncodedArtifactMode {
    if category.ends_with("-fragment") {
        TransformTemplateEncodedArtifactMode::Fragment
    } else {
        TransformTemplateEncodedArtifactMode::Document
    }
}

fn conversion_output_insertion_context(
    target: &TransformTemplateEncodingTarget,
    output_contract: &ConversionOutputContractDescriptor,
    options: &TransformTemplateEncodeOptions,
    produces: TransformTemplateOutputProducedKind,
) -> TransformTemplateEncodedArtifactInsertionContext {
    let mut context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        target,
        Some(produces),
    );
    context.formatter_profile = output_contract.formatter_profile.clone();
    context.color_profile = output_contract.color_profile.clone();
    context.mode = Some(options.mode);
    context.canonical = Some(options.canonical);
    context.source_map_policy = Some(options.source_map_policy);
    context
}

fn conversion_output_pipeline(
    output_contract: &ConversionOutputContractDescriptor,
    writer_options: &TransformTemplateEncodeOptions,
    writer_insertion_context: &TransformTemplateEncodedArtifactInsertionContext,
) -> ConversionOutputPipeline {
    let cemt_target =
        TransformTemplateEncodingTarget::new(CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, "cem-tree");
    let formatter_profile = conversion_cem_tree_formatter_profile(output_contract);
    let color_profile = conversion_cem_tree_color_profile(output_contract);
    let cemt_options =
        conversion_cem_tree_pipeline_options(writer_options, &formatter_profile, &color_profile);
    let cemt_insertion_context = conversion_cem_tree_insertion_context(
        &cemt_target,
        &cemt_options,
        &formatter_profile,
        &color_profile,
    );

    ConversionOutputPipeline {
        stages: vec![
            ConversionOutputPipelineStage::Transform,
            ConversionOutputPipelineStage::Format,
            ConversionOutputPipelineStage::Color,
            ConversionOutputPipelineStage::Writer,
        ],
        cemt_target,
        cemt_options,
        cemt_insertion_context,
        cemt_produces: TransformTemplateOutputProducedKind::CemTree,
        writer_insertion_context: writer_insertion_context.clone(),
        writer_produces: writer_insertion_context
            .produces
            .unwrap_or(TransformTemplateOutputProducedKind::Text),
    }
}

pub fn direct_cem_output_pipeline() -> ConversionOutputPipeline {
    let output_contract = ConversionOutputContractDescriptor {
        output_syntax: Some(ConversionOutputSyntax::Text),
        encoding_category: Some("cem-document".to_owned()),
        formatter_profile: Some("compact".to_owned()),
        color_profile: Some("none".to_owned()),
        parity: None,
    };
    let writer_target = TransformTemplateEncodingTarget::new(
        CEM_ML_CONTENT_TYPE,
        CEM_ML_SCHEMA_URI,
        "cem-document",
    );
    let writer_options = TransformTemplateEncodeOptions {
        formatter_profile: output_contract.formatter_profile.clone(),
        color_profile: output_contract.color_profile.clone(),
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: true,
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };
    let writer_insertion_context = conversion_output_insertion_context(
        &writer_target,
        &output_contract,
        &writer_options,
        TransformTemplateOutputProducedKind::Text,
    );
    conversion_output_pipeline(&output_contract, &writer_options, &writer_insertion_context)
}

pub fn direct_html_output_pipeline() -> ConversionOutputPipeline {
    direct_markup_output_pipeline(
        ConversionOutputSyntax::Html,
        HTML_CONTENT_TYPE,
        HTML_SCHEMA_URI,
        "html-document",
        Some("classes"),
    )
}

pub fn direct_xml_output_pipeline() -> ConversionOutputPipeline {
    direct_markup_output_pipeline(
        ConversionOutputSyntax::Xml,
        XML_CONTENT_TYPE,
        XML_SCHEMA_URI,
        "xml-document",
        None,
    )
}

fn direct_markup_output_pipeline(
    output_syntax: ConversionOutputSyntax,
    content_type: &str,
    schema: &str,
    category: &str,
    color_profile: Option<&str>,
) -> ConversionOutputPipeline {
    let output_contract = ConversionOutputContractDescriptor {
        output_syntax: Some(output_syntax),
        encoding_category: Some(category.to_owned()),
        formatter_profile: Some("compact".to_owned()),
        color_profile: color_profile.map(str::to_owned),
        parity: None,
    };
    let writer_target = TransformTemplateEncodingTarget::new(content_type, schema, category);
    let writer_options = TransformTemplateEncodeOptions {
        formatter_profile: output_contract.formatter_profile.clone(),
        color_profile: output_contract.color_profile.clone(),
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: true,
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };
    let writer_insertion_context = conversion_output_insertion_context(
        &writer_target,
        &output_contract,
        &writer_options,
        TransformTemplateOutputProducedKind::Text,
    );
    conversion_output_pipeline(&output_contract, &writer_options, &writer_insertion_context)
}

fn conversion_cem_tree_pipeline_options(
    writer_options: &TransformTemplateEncodeOptions,
    formatter_profile: &str,
    color_profile: &str,
) -> TransformTemplateEncodeOptions {
    TransformTemplateEncodeOptions {
        formatter: None,
        formatter_profile: Some(formatter_profile.to_owned()),
        colorizer: None,
        color_profile: Some(color_profile.to_owned()),
        mode: writer_options.mode,
        canonical: writer_options.canonical,
        line_ending: writer_options.line_ending.clone(),
        ordering: writer_options.ordering.clone(),
        wrap_column: writer_options.wrap_column.clone(),
        formatter_options: writer_options.formatter_options.clone(),
        indent: writer_options.indent.clone(),
        tab_size: writer_options.tab_size.clone(),
        source_map_policy: writer_options.source_map_policy,
        ..TransformTemplateEncodeOptions::default()
    }
}

fn conversion_cem_tree_insertion_context(
    target: &TransformTemplateEncodingTarget,
    options: &TransformTemplateEncodeOptions,
    formatter_profile: &str,
    color_profile: &str,
) -> TransformTemplateEncodedArtifactInsertionContext {
    let mut context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        target,
        Some(TransformTemplateOutputProducedKind::CemTree),
    );
    context.formatter_profile = Some(formatter_profile.to_owned());
    context.color_profile = Some(color_profile.to_owned());
    context.mode = Some(options.mode);
    context.canonical = Some(options.canonical);
    context.source_map_policy = Some(options.source_map_policy);
    context
}

fn conversion_cem_tree_formatter_profile(
    output_contract: &ConversionOutputContractDescriptor,
) -> String {
    output_contract
        .formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(|profile| match profile {
            "compact" | "pretty" | "tabular" => profile,
            "format-tree" | "cem.format-tree" | "canonical" => "compact",
            _ => "compact",
        })
        .unwrap_or("compact")
        .to_owned()
}

fn conversion_cem_tree_color_profile(
    output_contract: &ConversionOutputContractDescriptor,
) -> String {
    output_contract
        .color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("none")
        .to_owned()
}

fn conversion_output_color_profile(
    descriptor: &ConversionDescriptor,
    output_contract: &ConversionOutputContractDescriptor,
    expected_syntax: TransformTemplateTargetSyntaxKind,
    category: &str,
) -> Result<Option<TransformTemplateColorOutputProfile>, Diagnostic> {
    let Some(selector) = output_contract
        .color_profile
        .as_deref()
        .map(str::trim)
        .filter(|selector| !selector.is_empty())
    else {
        return Ok(None);
    };

    if matches!(selector, "none" | "plain" | "md" | "markdown") {
        let profile = TransformTemplateColorOutputProfile::plain();
        return profile.validate().map(|_| Some(profile)).map_err(|error| {
            conversion_output_safety_diagnostic(
                CONVERSION_OUTPUT_COLOR_PROFILE_UNSAFE_CODE,
                format!(
                    "converter `{}` has unsafe color profile `{}`: {}",
                    descriptor.id, selector, error
                ),
            )
        });
    }

    if TransformTemplateHtmlColorMode::parse(selector).is_some() {
        if expected_syntax != TransformTemplateTargetSyntaxKind::Html {
            return Err(conversion_output_safety_diagnostic(
                CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE,
                format!(
                    "converter `{}` HTML color profile `{}` cannot be used for `{}` output",
                    descriptor.id,
                    selector,
                    expected_syntax.as_str()
                ),
            ));
        }
        let profile =
            TransformTemplateColorOutputProfile::html_from_selector(selector).map_err(|error| {
                conversion_output_safety_diagnostic(
                    CONVERSION_OUTPUT_COLOR_PROFILE_UNSAFE_CODE,
                    format!(
                        "converter `{}` has unsafe HTML color profile `{}`: {}",
                        descriptor.id, selector, error
                    ),
                )
            })?;
        return profile.validate().map(|_| Some(profile)).map_err(|error| {
            conversion_output_safety_diagnostic(
                CONVERSION_OUTPUT_COLOR_PROFILE_UNSAFE_CODE,
                format!(
                    "converter `{}` has unsafe HTML color profile `{}`: {}",
                    descriptor.id, selector, error
                ),
            )
        });
    }

    if TransformTemplateTerminalColorCapability::parse(selector).is_some() {
        if expected_syntax != TransformTemplateTargetSyntaxKind::Text
            || !category.starts_with("terminal-")
        {
            return Err(conversion_output_safety_diagnostic(
                CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE,
                format!(
                    "converter `{}` terminal color profile `{}` requires a terminal text encoding category",
                    descriptor.id, selector
                ),
            ));
        }
        let profile = TransformTemplateColorOutputProfile::terminal_from_selector(selector)
            .map_err(|error| {
                conversion_output_safety_diagnostic(
                    CONVERSION_OUTPUT_COLOR_PROFILE_UNSAFE_CODE,
                    format!(
                        "converter `{}` has unsafe terminal color profile `{}`: {}",
                        descriptor.id, selector, error
                    ),
                )
            })?;
        return profile.validate().map(|_| Some(profile)).map_err(|error| {
            conversion_output_safety_diagnostic(
                CONVERSION_OUTPUT_COLOR_PROFILE_UNSAFE_CODE,
                format!(
                    "converter `{}` has unsafe terminal color profile `{}`: {}",
                    descriptor.id, selector, error
                ),
            )
        });
    }

    Err(conversion_output_safety_diagnostic(
        CONVERSION_OUTPUT_COLOR_PROFILE_UNSAFE_CODE,
        format!(
            "converter `{}` declares unknown color profile `{}`",
            descriptor.id, selector
        ),
    ))
}

fn conversion_encoding_category_syntax(
    category: &str,
) -> Option<TransformTemplateTargetSyntaxKind> {
    let category = category.trim();
    if category.starts_with("cem-bin-") {
        Some(TransformTemplateTargetSyntaxKind::Binary)
    } else if category.starts_with("html-") {
        Some(TransformTemplateTargetSyntaxKind::Html)
    } else if category.starts_with("xml-") {
        Some(TransformTemplateTargetSyntaxKind::Xml)
    } else if category.starts_with("json-") || category.starts_with("ai-") {
        Some(TransformTemplateTargetSyntaxKind::Json)
    } else if category.starts_with("yaml-") {
        Some(TransformTemplateTargetSyntaxKind::Yaml)
    } else if category.starts_with("csv-") {
        Some(TransformTemplateTargetSyntaxKind::Csv)
    } else if category.starts_with("markdown-") {
        Some(TransformTemplateTargetSyntaxKind::Markdown)
    } else if category.starts_with("css-") {
        Some(TransformTemplateTargetSyntaxKind::Css)
    } else if category.starts_with("terminal-")
        || category.starts_with("cem-ql-")
        || category.starts_with("rnc-")
        || category == "text"
        || category.starts_with("text-")
    {
        Some(TransformTemplateTargetSyntaxKind::Text)
    } else if category.starts_with("cemt-") || category.starts_with("cem-") {
        Some(TransformTemplateTargetSyntaxKind::Cemt)
    } else {
        None
    }
}

fn conversion_template_syntax_kind(
    output_syntax: ConversionOutputSyntax,
) -> TransformTemplateTargetSyntaxKind {
    match output_syntax {
        ConversionOutputSyntax::Html => TransformTemplateTargetSyntaxKind::Html,
        ConversionOutputSyntax::Xml => TransformTemplateTargetSyntaxKind::Xml,
        ConversionOutputSyntax::Json => TransformTemplateTargetSyntaxKind::Json,
        ConversionOutputSyntax::Yaml => TransformTemplateTargetSyntaxKind::Yaml,
        ConversionOutputSyntax::Csv => TransformTemplateTargetSyntaxKind::Csv,
        ConversionOutputSyntax::Css => TransformTemplateTargetSyntaxKind::Css,
        ConversionOutputSyntax::Markdown => TransformTemplateTargetSyntaxKind::Markdown,
        ConversionOutputSyntax::Cemt => TransformTemplateTargetSyntaxKind::Cemt,
        ConversionOutputSyntax::Text => TransformTemplateTargetSyntaxKind::Text,
        ConversionOutputSyntax::Binary => TransformTemplateTargetSyntaxKind::Binary,
        ConversionOutputSyntax::Opaque => TransformTemplateTargetSyntaxKind::Opaque,
    }
}

fn conversion_output_produced_kind(
    syntax: TransformTemplateTargetSyntaxKind,
) -> TransformTemplateOutputProducedKind {
    match syntax {
        TransformTemplateTargetSyntaxKind::Binary => TransformTemplateOutputProducedKind::Bytes,
        _ => TransformTemplateOutputProducedKind::Text,
    }
}

fn conversion_output_syntax_selector(output_syntax: ConversionOutputSyntax) -> &'static str {
    match output_syntax {
        ConversionOutputSyntax::Html => "html",
        ConversionOutputSyntax::Xml => "xml",
        ConversionOutputSyntax::Json => "json",
        ConversionOutputSyntax::Yaml => "yaml",
        ConversionOutputSyntax::Csv => "csv",
        ConversionOutputSyntax::Css => "css",
        ConversionOutputSyntax::Markdown => "markdown",
        ConversionOutputSyntax::Cemt => "cemt",
        ConversionOutputSyntax::Text => "text",
        ConversionOutputSyntax::Binary => "binary",
        ConversionOutputSyntax::Opaque => "opaque",
    }
}

fn conversion_output_safety_diagnostic(code: &'static str, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        ..Diagnostic::default()
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
        if let Some(planning_domain) = options.planning_domain {
            return descriptor.planning_domain() == planning_domain;
        }
        return true;
    }
    if !descriptor.implicit || descriptor.explicit_only {
        return false;
    }
    if let Some(planning_domain) = options.planning_domain {
        return descriptor.planning_domain() == planning_domain;
    }
    true
}

fn descriptor_rank(descriptor: &ConversionDescriptor) -> (u32, u8) {
    let implementation_rank = match descriptor.implementation {
        ConversionImplementation::Cemt => 0,
        ConversionImplementation::Rust => 1,
    };
    (descriptor.cost, implementation_rank)
}

fn descriptor_is_schema_output_producer(descriptor: &ConversionDescriptor) -> bool {
    let Some(source_schema) = descriptor.from.schema.as_deref() else {
        return false;
    };
    let Some(target_schema) = descriptor.to.schema.as_deref() else {
        return false;
    };

    is_canonical_projection_schema(source_schema) && !is_canonical_projection_schema(target_schema)
}

fn is_canonical_projection_schema(schema: &str) -> bool {
    matches!(
        schema,
        CEM_AST_PROJECTION_SCHEMA_URI
            | CEM_DOM_PROJECTION_SCHEMA_URI
            | CEM_EVENTS_PROJECTION_SCHEMA_URI
    )
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

pub fn conversion_descriptors_from_schema_package(
    package: &BuiltinSchemaPackage,
) -> Result<Vec<ConversionDescriptor>, ConversionManifestError> {
    let base_path = package_manifest_base_path(&package.descriptor);
    conversion_descriptors_from_schema_package_manifest(
        &package.descriptor.package_id,
        package.manifest_source,
        &base_path,
    )
}

pub fn conversion_descriptors_from_schema_package_manifest(
    package_id_hint: &str,
    manifest_source: &str,
    base_path: &str,
) -> Result<Vec<ConversionDescriptor>, ConversionManifestError> {
    conversion_descriptors_from_schema_package_manifest_inner(
        package_id_hint,
        manifest_source,
        base_path,
        true,
    )
}

pub fn conversion_descriptors_from_validated_schema_package_manifest(
    package_id_hint: &str,
    manifest_source: &str,
    base_path: &str,
) -> Result<Vec<ConversionDescriptor>, ConversionManifestError> {
    conversion_descriptors_from_schema_package_manifest_inner(
        package_id_hint,
        manifest_source,
        base_path,
        false,
    )
}

fn conversion_descriptors_from_schema_package_manifest_inner(
    package_id_hint: &str,
    manifest_source: &str,
    base_path: &str,
    validate_embedded_converter_templates: bool,
) -> Result<Vec<ConversionDescriptor>, ConversionManifestError> {
    let document = parse_cem_document(manifest_source);
    let package_id = package_manifest_package_id(package_id_hint, &document)?;
    let Some(package_node_id) = first_element_id_by_local_name(&document, "package") else {
        return Err(ConversionManifestError::MissingPackageElement);
    };

    let mut descriptors = Vec::new();
    for converter_id in element_child_ids_by_local_name(&document, package_node_id, "converter") {
        let Some(descriptor) = conversion_descriptor_from_manifest_node(
            &document,
            converter_id,
            &package_id,
            base_path,
        ) else {
            continue;
        };
        descriptors.push(descriptor);
    }
    if validate_embedded_converter_templates && builtin_schema_package_source(&package_id).is_some()
    {
        for descriptor in &descriptors {
            validate_conversion_descriptor_cemt_output_template_contract(descriptor)?;
        }
    }
    Ok(descriptors)
}

pub fn conversion_package_artifacts_from_schema_package(
    package: &BuiltinSchemaPackage,
) -> Result<Vec<ConversionPackageArtifactDescriptor>, ConversionManifestError> {
    let base_path = package_manifest_base_path(&package.descriptor);
    conversion_package_artifacts_from_schema_package_manifest(
        &package.descriptor.package_id,
        package.manifest_source,
        &base_path,
    )
}

pub fn conversion_package_artifacts_from_schema_package_manifest(
    package_id_hint: &str,
    manifest_source: &str,
    base_path: &str,
) -> Result<Vec<ConversionPackageArtifactDescriptor>, ConversionManifestError> {
    conversion_package_artifacts_from_schema_package_manifest_inner(
        package_id_hint,
        manifest_source,
        base_path,
        true,
    )
}

pub fn conversion_package_artifacts_from_validated_schema_package_manifest(
    package_id_hint: &str,
    manifest_source: &str,
    base_path: &str,
) -> Result<Vec<ConversionPackageArtifactDescriptor>, ConversionManifestError> {
    conversion_package_artifacts_from_schema_package_manifest_inner(
        package_id_hint,
        manifest_source,
        base_path,
        false,
    )
}

fn conversion_package_artifacts_from_schema_package_manifest_inner(
    package_id_hint: &str,
    manifest_source: &str,
    base_path: &str,
    validate_embedded_artifact_contracts: bool,
) -> Result<Vec<ConversionPackageArtifactDescriptor>, ConversionManifestError> {
    let document = parse_cem_document(manifest_source);
    let package_id = package_manifest_package_id(package_id_hint, &document)?;
    let Some(package_node_id) = first_element_id_by_local_name(&document, "package") else {
        return Err(ConversionManifestError::MissingPackageElement);
    };

    let mut artifacts = Vec::new();
    for artifact_node_id in element_child_ids_by_local_name(&document, package_node_id, "artifact")
    {
        let Some(artifact) = conversion_package_artifact_from_manifest_node(
            &document,
            artifact_node_id,
            &package_id,
            base_path,
        ) else {
            continue;
        };
        artifacts.push(artifact);
    }
    if validate_embedded_artifact_contracts {
        for artifact in &artifacts {
            validate_conversion_package_artifact_cemt_contract(artifact, &artifacts)?;
        }
    }
    Ok(artifacts)
}

fn conversion_package_artifact_from_manifest_node(
    document: &CemDocument,
    node_id: AstNodeId,
    package_id: &str,
    base_path: &str,
) -> Option<ConversionPackageArtifactDescriptor> {
    let attrs = collect_manifest_attrs(document, node_id);
    let kind = optional_manifest_attr(&attrs, "kind")?.to_owned();
    let path = package_relative_path(base_path, optional_manifest_attr(&attrs, "path")?);
    let content_type = optional_manifest_attr(&attrs, "content-type").map(content_type_essence);
    let schema = optional_manifest_attr(&attrs, "schema").map(str::to_owned);
    let target_content_type =
        optional_manifest_attr(&attrs, "target-content-type").map(content_type_essence);
    let target_schema = optional_manifest_attr(&attrs, "target-schema").map(str::to_owned);
    let target_category = optional_manifest_attr(&attrs, "target-category").map(str::to_owned);
    let generated = parse_manifest_bool(&attrs, "generated").unwrap_or(false);
    let artifact = ConversionPackageArtifactDescriptor {
        package_id: package_id.to_owned(),
        kind,
        path,
        content_type,
        schema,
        target_content_type,
        target_schema,
        target_category,
        function_name: optional_manifest_attr(&attrs, "function-name").map(str::to_owned),
        function_profile: optional_manifest_attr(&attrs, "function-profile").map(str::to_owned),
        formatter_profile: optional_manifest_attr(&attrs, "formatter-profile").map(str::to_owned),
        color_profile: optional_manifest_attr(&attrs, "color-profile").map(str::to_owned),
        generated,
    };
    Some(artifact)
}

fn validate_conversion_package_artifact_cemt_contract(
    artifact: &ConversionPackageArtifactDescriptor,
    package_artifacts: &[ConversionPackageArtifactDescriptor],
) -> Result<(), ConversionManifestError> {
    let Some(function_name) = artifact.function_name.as_deref() else {
        return Ok(());
    };
    let Some(source) = builtin_schema_package_artifact_source(&artifact.package_id, &artifact.path)
    else {
        return Err(conversion_artifact_contract_error(
            artifact,
            "referenced CEMT artifact source is not embedded",
        ));
    };

    let parse_response =
        parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
            template: TemplateInput {
                uri: artifact.path.clone(),
                bytes: source.source.as_bytes().to_vec(),
                identity: Some(FormatIdentity {
                    content_type: artifact.content_type.clone(),
                    schema: artifact.schema.clone(),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
        });
    if let Some(diagnostic) = parse_response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(conversion_artifact_contract_error(
            artifact,
            format!("CEMT source failed to parse: {}", diagnostic.message),
        ));
    }

    let Some(function) = parse_response
        .module_options
        .output_functions
        .iter()
        .find(|function| function.name == function_name)
    else {
        return Err(conversion_artifact_contract_error(
            artifact,
            format!("no CEMT output function named `{function_name}` is declared"),
        ));
    };

    if let Some(mismatch) = validate_transform_template_artifact_function_contract(
        function,
        TransformTemplateArtifactFunctionContract {
            artifact_kind: Some(artifact.kind.as_str()),
            target_content_type: artifact.target_content_type.as_deref(),
            target_schema: artifact.target_schema.as_deref(),
            target_category: artifact.target_category.as_deref(),
            function_profile: artifact.function_profile.as_deref(),
        },
    )
    .into_iter()
    .next()
    {
        return Err(conversion_artifact_contract_mismatch(
            artifact,
            mismatch.field,
            mismatch.expected.as_str(),
            mismatch.actual.as_str(),
        ));
    }
    validate_conversion_package_artifact_cem_tree_stage_metadata_contract(
        artifact,
        function,
        &parse_response.module_options,
        package_artifacts,
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CemtStageMetadataContract {
    Formatter,
    Colorizer,
}

fn validate_conversion_package_artifact_cem_tree_stage_metadata_contract(
    artifact: &ConversionPackageArtifactDescriptor,
    function: &TransformTemplateOutputFunctionDescriptor,
    module_options: &TransformTemplateModuleOptions,
    package_artifacts: &[ConversionPackageArtifactDescriptor],
) -> Result<(), ConversionManifestError> {
    let Some(contract) =
        conversion_package_artifact_cem_tree_stage_metadata_contract(artifact, function)
    else {
        return Ok(());
    };

    let reachable_body = conversion_package_artifact_reachable_cemt_body(
        artifact,
        function.name.as_str(),
        module_options,
        package_artifacts,
        contract,
    )?;
    let missing = conversion_package_artifact_cem_tree_stage_missing_metadata_terms(
        contract,
        &reachable_body,
    );
    if !missing.is_empty() {
        return Err(conversion_artifact_contract_error(
            artifact,
            format!(
                "{} CEMT artifact does not build required {} CEM tree metadata: {}",
                contract.stage_label(),
                contract.output_label(),
                missing.join(", ")
            ),
        ));
    }

    Ok(())
}

fn conversion_package_artifact_cem_tree_stage_metadata_contract(
    artifact: &ConversionPackageArtifactDescriptor,
    function: &TransformTemplateOutputFunctionDescriptor,
) -> Option<CemtStageMetadataContract> {
    if !conversion_package_artifact_targets_cem_tree(artifact)
        || function.produces != TransformTemplateOutputProducedKind::CemTree
    {
        return None;
    }

    match function.kind {
        TransformTemplateOutputFunctionKind::Format
            if CemtStageMetadataContract::Formatter
                .includes_artifact_kind(artifact.kind.as_str()) =>
        {
            Some(CemtStageMetadataContract::Formatter)
        }
        TransformTemplateOutputFunctionKind::Color
            if CemtStageMetadataContract::Colorizer
                .includes_artifact_kind(artifact.kind.as_str()) =>
        {
            Some(CemtStageMetadataContract::Colorizer)
        }
        _ => None,
    }
}

fn conversion_package_artifact_targets_cem_tree(
    artifact: &ConversionPackageArtifactDescriptor,
) -> bool {
    artifact
        .target_content_type
        .as_deref()
        .map(content_type_essence)
        .as_deref()
        == Some(CEM_ML_CONTENT_TYPE)
        && artifact.target_schema.as_deref() == Some(CEM_ML_SCHEMA_URI)
        && artifact.target_category.as_deref() == Some("cem-tree")
}

impl CemtStageMetadataContract {
    fn stage_label(self) -> &'static str {
        match self {
            Self::Formatter => "formatter",
            Self::Colorizer => "colorizer",
        }
    }

    fn output_label(self) -> &'static str {
        match self {
            Self::Formatter => "formatted",
            Self::Colorizer => "colored",
        }
    }

    fn artifact_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Formatter => CEM_TREE_FORMATTER_STAGE_ARTIFACT_KINDS,
            Self::Colorizer => CEM_TREE_COLORIZER_STAGE_ARTIFACT_KINDS,
        }
    }

    fn from_artifact_kind(kind: &str) -> Option<Self> {
        let kind = kind.trim();
        if Self::Formatter.includes_artifact_kind(kind) {
            Some(Self::Formatter)
        } else if Self::Colorizer.includes_artifact_kind(kind) {
            Some(Self::Colorizer)
        } else {
            None
        }
    }

    fn from_function_kind(kind: TransformTemplateOutputFunctionKind) -> Option<Self> {
        match kind {
            TransformTemplateOutputFunctionKind::Format => Some(Self::Formatter),
            TransformTemplateOutputFunctionKind::Color => Some(Self::Colorizer),
            TransformTemplateOutputFunctionKind::Encoding => None,
        }
    }

    fn function_kind(self) -> TransformTemplateOutputFunctionKind {
        match self {
            Self::Formatter => TransformTemplateOutputFunctionKind::Format,
            Self::Colorizer => TransformTemplateOutputFunctionKind::Color,
        }
    }

    fn helper_artifact_kind(self) -> &'static str {
        match self {
            Self::Formatter => CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND,
            Self::Colorizer => CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND,
        }
    }

    fn includes_artifact_kind(self, kind: &str) -> bool {
        self.artifact_kinds().contains(&kind)
    }

    fn required_metadata_terms(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            Self::Formatter => &[
                ("formatterProfile", &["formatterProfile"]),
                ("formatNodes", &["formatNodes", "appendFormatNode"]),
                ("cem.format-tree", &["cem.format-tree"]),
                ("format-marker", &["format-marker"]),
                ("format-decision", &["format-decision"]),
                ("formatterRole", &["formatterRole"]),
            ],
            Self::Colorizer => &[
                ("colored", &["colored"]),
                ("colorProfile", &["colorProfile"]),
                ("colorNodes", &["colorNodes", "appendColorNode"]),
                ("cem.color-tree", &["cem.color-tree"]),
                ("color-marker", &["color-marker"]),
                ("color-decision", &["color-decision"]),
                ("colorizerRole", &["colorizerRole"]),
            ],
        }
    }
}

fn conversion_package_artifact_reachable_cemt_body(
    artifact: &ConversionPackageArtifactDescriptor,
    root_function_name: &str,
    module_options: &TransformTemplateModuleOptions,
    package_artifacts: &[ConversionPackageArtifactDescriptor],
    contract: CemtStageMetadataContract,
) -> Result<String, ConversionManifestError> {
    let mut bodies = BTreeMap::new();
    collect_conversion_package_artifact_cemt_bodies(module_options, &mut bodies);

    let mut loaded_artifacts = BTreeSet::new();
    loaded_artifacts.insert((
        artifact.package_id.clone(),
        artifact.path.clone(),
        artifact.function_name.clone(),
    ));
    for related in package_artifacts.iter().filter(|candidate| {
        conversion_package_artifact_is_related_cem_tree_stage_artifact(
            artifact, candidate, contract,
        )
    }) {
        if !loaded_artifacts.insert((
            related.package_id.clone(),
            related.path.clone(),
            related.function_name.clone(),
        )) {
            continue;
        }
        let Some(source) =
            builtin_schema_package_artifact_source(&related.package_id, &related.path)
        else {
            continue;
        };
        let parse_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: related.path.clone(),
                    bytes: source.source.as_bytes().to_vec(),
                    identity: Some(FormatIdentity {
                        content_type: related.content_type.clone(),
                        schema: related.schema.clone(),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });
        if let Some(diagnostic) = parse_response
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            return Err(conversion_artifact_contract_error(
                artifact,
                format!(
                    "related `{}` CEMT artifact `{}` failed to parse: {}",
                    related.kind, related.path, diagnostic.message
                ),
            ));
        }
        collect_conversion_package_artifact_cemt_bodies(
            &parse_response.module_options,
            &mut bodies,
        );
    }

    let mut reachable = String::new();
    let mut visited = BTreeSet::new();
    let mut pending = vec![root_function_name.to_owned()];
    while let Some(function_name) = pending.pop() {
        if !visited.insert(function_name.clone()) {
            continue;
        }
        let Some(body) = bodies.get(&function_name) else {
            continue;
        };
        reachable.push('\n');
        reachable.push_str(body);
        for called in cemt_call_function_names(body) {
            if !visited.contains(&called) {
                pending.push(called);
            }
        }
    }
    Ok(reachable)
}

fn collect_conversion_package_artifact_cemt_bodies(
    module_options: &TransformTemplateModuleOptions,
    bodies: &mut BTreeMap<String, String>,
) {
    for function in &module_options.output_functions {
        if let Some(body) = function.body_expression.as_deref() {
            bodies
                .entry(function.name.clone())
                .or_insert_with(|| body.to_owned());
        }
    }
    for function in &module_options.functions {
        if let Some(body) = function.body_expression.as_deref() {
            bodies
                .entry(function.name.clone())
                .or_insert_with(|| body.to_owned());
        }
    }
}

fn conversion_package_artifact_is_related_cem_tree_stage_artifact(
    root: &ConversionPackageArtifactDescriptor,
    candidate: &ConversionPackageArtifactDescriptor,
    contract: CemtStageMetadataContract,
) -> bool {
    if root.package_id != candidate.package_id
        || !conversion_package_artifact_targets_cem_tree(candidate)
        || candidate.content_type.as_deref() != Some(CEM_TRANSFORM_CONTENT_TYPE)
        || candidate.schema.as_deref() != Some(CEM_TRANSFORM_SCHEMA_URI)
    {
        return false;
    }

    match contract {
        CemtStageMetadataContract::Formatter => {
            contract.includes_artifact_kind(candidate.kind.as_str())
                && conversion_package_artifact_profile_matches(
                    root.formatter_profile.as_deref(),
                    candidate.formatter_profile.as_deref(),
                )
        }
        CemtStageMetadataContract::Colorizer => {
            contract.includes_artifact_kind(candidate.kind.as_str())
                && conversion_package_artifact_profile_matches(
                    root.color_profile.as_deref(),
                    candidate.color_profile.as_deref(),
                )
        }
    }
}

fn conversion_package_artifact_profile_matches(
    root_profile: Option<&str>,
    candidate_profile: Option<&str>,
) -> bool {
    let root_profile = conversion_trimmed_non_empty(root_profile);
    conversion_trimmed_non_empty(candidate_profile)
        .is_none_or(|profile| Some(profile) == root_profile)
}

fn conversion_trimmed_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn conversion_package_artifact_cem_tree_stage_missing_metadata_terms(
    contract: CemtStageMetadataContract,
    body: &str,
) -> Vec<&'static str> {
    contract
        .required_metadata_terms()
        .iter()
        .filter_map(|(label, alternatives)| {
            (!alternatives.iter().any(|term| body.contains(term))).then_some(*label)
        })
        .collect()
}

fn cemt_call_function_names(expression: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut remainder = expression;
    while let Some(index) = remainder.find("call(") {
        let call = &remainder[index..];
        if let Some(name) = cemt_direct_call_function_name(call) {
            names.push(name);
        }
        remainder = &call["call(".len()..];
    }
    names
}

fn conversion_artifact_contract_mismatch(
    artifact: &ConversionPackageArtifactDescriptor,
    field: &str,
    expected: &str,
    actual: &str,
) -> ConversionManifestError {
    conversion_artifact_contract_error(
        artifact,
        format!("{field} metadata expected `{expected}`, CEMT declares `{actual}`"),
    )
}

fn conversion_artifact_contract_error(
    artifact: &ConversionPackageArtifactDescriptor,
    message: impl Into<String>,
) -> ConversionManifestError {
    ConversionManifestError::ArtifactContract {
        package_id: artifact.package_id.clone(),
        path: artifact.path.clone(),
        message: message.into(),
    }
}

fn validate_conversion_descriptor_cemt_output_template_contract(
    descriptor: &ConversionDescriptor,
) -> Result<(), ConversionManifestError> {
    if !conversion_descriptor_claims_formatted_cem_tree_pipeline(descriptor) {
        return Ok(());
    }
    let Some(template) = descriptor.template.as_ref() else {
        return Err(conversion_converter_template_contract_error(
            descriptor,
            "",
            "CEMT converter declares a formatter/coloring output pipeline but has no template",
        ));
    };
    if content_type_essence(&template.content_type) != CEM_TRANSFORM_CONTENT_TYPE {
        return Err(conversion_converter_template_contract_error(
            descriptor,
            &template.path,
            format!(
                "formatter/coloring output pipeline requires CEMT content type `{}`, manifest declares `{}`",
                CEM_TRANSFORM_CONTENT_TYPE, template.content_type
            ),
        ));
    }
    if template.schema.as_deref() != Some(CEM_TRANSFORM_SCHEMA_URI) {
        return Err(conversion_converter_template_contract_error(
            descriptor,
            &template.path,
            format!(
                "formatter/coloring output pipeline requires CEMT schema `{}`",
                CEM_TRANSFORM_SCHEMA_URI
            ),
        ));
    }

    let Some(source) =
        builtin_schema_package_artifact_source(&descriptor.package_id, &template.path)
    else {
        return Err(conversion_converter_template_contract_error(
            descriptor,
            &template.path,
            "built-in CEMT converter declares a formatter/coloring output pipeline, but its template source is not embedded in the schema package source catalog",
        ));
    };

    validate_conversion_descriptor_cemt_output_template_source(descriptor, template, source.source)
}

fn conversion_descriptor_claims_formatted_cem_tree_pipeline(
    descriptor: &ConversionDescriptor,
) -> bool {
    descriptor.implementation == ConversionImplementation::Cemt
        && descriptor.output_contract.output_syntax.is_some()
        && conversion_trimmed_non_empty(descriptor.output_contract.encoding_category.as_deref())
            .is_some()
        && (conversion_trimmed_non_empty(descriptor.output_contract.formatter_profile.as_deref())
            .is_some()
            || conversion_trimmed_non_empty(descriptor.output_contract.color_profile.as_deref())
                .is_some())
        && descriptor.to.schema.is_some()
}

fn validate_conversion_descriptor_cemt_output_template_source(
    descriptor: &ConversionDescriptor,
    template: &ConversionTemplateDescriptor,
    source: &str,
) -> Result<(), ConversionManifestError> {
    let template_input = TemplateInput {
        uri: template.path.clone(),
        bytes: source.as_bytes().to_vec(),
        identity: Some(FormatIdentity {
            content_type: Some(template.content_type.clone()),
            schema: template.schema.clone(),
            ..FormatIdentity::default()
        }),
        root_scope: ScopeConfig::default(),
    };
    let entrypoint = template
        .entrypoint
        .as_deref()
        .map(TransformTemplateEntrypoint::named)
        .unwrap_or_else(TransformTemplateEntrypoint::implicit);
    let params = BTreeMap::new();
    let data_bindings = vec!["input".to_owned()];
    let adapter = DomProjectionParityCemtAdapter;
    let compile_response = adapter
        .compile(TransformTemplateCompileRequest {
            template: &template_input,
            entrypoint: &entrypoint,
            params: &params,
            data_bindings: &data_bindings,
            module_options: TransformTemplateModuleOptions::default(),
            module_preflight: TransformTemplateModulePreflight::default(),
            execution_policy: TransformExecutionPolicy::default(),
        })
        .map_err(|error| {
            conversion_converter_template_contract_error(
                descriptor,
                &template.path,
                format!(
                    "formatter/coloring output pipeline requires a CEMT converter template that can render a formatted CEM tree before the writer: {error}"
                ),
            )
        })?;

    if let Some(diagnostic) = compile_response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(conversion_converter_template_contract_error(
            descriptor,
            &template.path,
            format!(
                "formatter/coloring output pipeline template compile emitted hard diagnostic `{}`",
                diagnostic.code
            ),
        ));
    }

    Ok(())
}

fn conversion_converter_template_contract_error(
    descriptor: &ConversionDescriptor,
    path: &str,
    message: impl Into<String>,
) -> ConversionManifestError {
    ConversionManifestError::ConverterTemplateContract {
        package_id: descriptor.package_id.clone(),
        converter_id: descriptor.id.clone(),
        path: path.to_owned(),
        message: message.into(),
    }
}

fn conversion_descriptor_from_manifest_node(
    document: &CemDocument,
    node_id: AstNodeId,
    package_id: &str,
    base_path: &str,
) -> Option<ConversionDescriptor> {
    let attrs = collect_manifest_attrs(document, node_id);
    let id = optional_manifest_attr(&attrs, "id")?.to_owned();
    let implementation =
        optional_manifest_attr(&attrs, "implementation").and_then(parse_manifest_implementation)?;
    let from = manifest_endpoint(document, node_id, "from")?;
    let to = manifest_endpoint(document, node_id, "to")?;
    let readiness = optional_manifest_attr(&attrs, "readiness")
        .and_then(parse_manifest_readiness)
        .unwrap_or(ConversionReadiness::Ready);
    let streamable = parse_manifest_bool(&attrs, "streamable").unwrap_or(false);
    let explicit_only = parse_manifest_bool(&attrs, "explicit-only").unwrap_or(false);
    let implicit = parse_manifest_bool(&attrs, "implicit").unwrap_or(!explicit_only);
    let cost = parse_manifest_cost(&attrs).unwrap_or(100);
    let output_contract = ConversionOutputContractDescriptor {
        output_syntax: optional_manifest_attr(&attrs, "output-syntax")
            .and_then(parse_manifest_output_syntax),
        encoding_category: optional_manifest_attr(&attrs, "encoding-category").map(str::to_owned),
        formatter_profile: optional_manifest_attr(&attrs, "formatter-profile").map(str::to_owned),
        color_profile: optional_manifest_attr(&attrs, "color-profile").map(str::to_owned),
        parity: optional_manifest_attr(&attrs, "parity").and_then(parse_manifest_parity_mode),
    };
    let parity_fixtures = manifest_parity_fixtures(document, node_id, base_path);

    let template = match implementation {
        ConversionImplementation::Cemt => match (
            optional_manifest_attr(&attrs, "template"),
            optional_manifest_attr(&attrs, "template-content-type"),
            optional_manifest_attr(&attrs, "template-schema"),
        ) {
            (Some(template_path), Some(template_content_type), Some(template_schema)) => {
                Some(ConversionTemplateDescriptor {
                    path: package_relative_path(base_path, template_path),
                    content_type: content_type_essence(template_content_type),
                    schema: Some(template_schema.to_owned()),
                    entrypoint: optional_manifest_attr(&attrs, "template-entrypoint")
                        .map(str::to_owned),
                })
            }
            _ => None,
        },
        ConversionImplementation::Rust => None,
    };

    let rust_symbol = optional_manifest_attr(&attrs, "rust-symbol").map(str::to_owned);
    let (rust_symbol, rust_fallback) = match implementation {
        ConversionImplementation::Cemt => (
            None,
            match (
                rust_symbol,
                optional_manifest_attr(&attrs, "fallback-reason"),
            ) {
                (Some(rust_symbol), Some(reason)) => Some(ConversionRustFallbackDescriptor {
                    rust_symbol,
                    reason: reason.to_owned(),
                }),
                _ => None,
            },
        ),
        ConversionImplementation::Rust => (rust_symbol, None),
    };

    Some(ConversionDescriptor {
        id,
        package_id: package_id.to_owned(),
        from,
        to,
        implementation,
        readiness,
        template,
        rust_symbol,
        rust_fallback,
        streamable,
        lossiness: optional_manifest_attr(&attrs, "lossiness").map(str::to_owned),
        output_contract,
        parity_fixtures,
        implicit,
        explicit_only,
        cost,
    })
}

fn package_manifest_package_id(
    package_id_hint: &str,
    document: &CemDocument,
) -> Result<String, ConversionManifestError> {
    let Some(package_node_id) = first_element_id_by_local_name(document, "package") else {
        return Err(ConversionManifestError::MissingPackageElement);
    };
    let attrs = collect_manifest_attrs(document, package_node_id);
    Ok(optional_manifest_attr(&attrs, "id")
        .unwrap_or(package_id_hint)
        .to_owned())
}

fn package_manifest_base_path(descriptor: &SchemaDescriptor) -> String {
    descriptor
        .source
        .split_once("/schema/")
        .map(|(base, _)| base.to_owned())
        .unwrap_or_else(|| format!("schema-packages/{}/v1", descriptor.package_id))
}

fn package_relative_path(base_path: &str, path: &str) -> String {
    let path = path.trim();
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with("schema-packages/")
        || path.contains("://")
    {
        return path.to_owned();
    }
    format!(
        "{}/{}",
        base_path.trim_end_matches('/'),
        path.trim_start_matches("./")
    )
}

fn manifest_parity_fixtures(
    document: &CemDocument,
    converter_node_id: AstNodeId,
    base_path: &str,
) -> Vec<ConversionParityFixtureDescriptor> {
    let mut parity_fixtures = Vec::new();
    for fixture_node_id in
        element_child_ids_by_local_name(document, converter_node_id, "parity-fixture")
    {
        let attrs = collect_manifest_attrs(document, fixture_node_id);
        let Some(id) = optional_manifest_attr(&attrs, "id").map(str::to_owned) else {
            continue;
        };
        let Some(path_attr) = optional_manifest_attr(&attrs, "path") else {
            continue;
        };
        let path = package_relative_path(base_path, path_attr);
        let content_type = optional_manifest_attr(&attrs, "content-type").map(content_type_essence);
        let schema = optional_manifest_attr(&attrs, "schema").map(str::to_owned);
        let expected_diagnostic_codes = optional_manifest_attr(&attrs, "expected-diagnostics")
            .map(|value| {
                value
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        parity_fixtures.push(ConversionParityFixtureDescriptor {
            id,
            path,
            content_type,
            schema,
            expected_diagnostic_codes,
        });
    }

    parity_fixtures
}

fn manifest_endpoint(
    document: &CemDocument,
    converter_node_id: AstNodeId,
    endpoint_name: &'static str,
) -> Option<ConversionEndpoint> {
    let endpoint_id = element_child_ids_by_local_name(document, converter_node_id, endpoint_name)
        .into_iter()
        .next()?;
    let attrs = collect_manifest_attrs(document, endpoint_id);
    let content_type = optional_manifest_attr(&attrs, "content-type")?;
    Some(match optional_manifest_attr(&attrs, "schema") {
        Some(schema) => ConversionEndpoint::with_schema(content_type, schema),
        None => ConversionEndpoint::new(content_type),
    })
}

fn parse_manifest_implementation(value: &str) -> Option<ConversionImplementation> {
    match value.trim() {
        "cemt" => Some(ConversionImplementation::Cemt),
        "rust" => Some(ConversionImplementation::Rust),
        _ => None,
    }
}

fn parse_manifest_readiness(value: &str) -> Option<ConversionReadiness> {
    match value.trim() {
        "ready" => Some(ConversionReadiness::Ready),
        "planned" => Some(ConversionReadiness::Planned),
        _ => None,
    }
}

fn parse_manifest_output_syntax(value: &str) -> Option<ConversionOutputSyntax> {
    match value.trim() {
        "html" => Some(ConversionOutputSyntax::Html),
        "xml" => Some(ConversionOutputSyntax::Xml),
        "json" => Some(ConversionOutputSyntax::Json),
        "yaml" => Some(ConversionOutputSyntax::Yaml),
        "csv" => Some(ConversionOutputSyntax::Csv),
        "css" => Some(ConversionOutputSyntax::Css),
        "markdown" => Some(ConversionOutputSyntax::Markdown),
        "cemt" => Some(ConversionOutputSyntax::Cemt),
        "text" => Some(ConversionOutputSyntax::Text),
        "binary" => Some(ConversionOutputSyntax::Binary),
        "opaque" => Some(ConversionOutputSyntax::Opaque),
        _ => None,
    }
}

fn parse_manifest_parity_mode(value: &str) -> Option<ConversionParityMode> {
    match value.trim() {
        "byte-exact" => Some(ConversionParityMode::ByteExact),
        "token-equivalent" => Some(ConversionParityMode::TokenEquivalent),
        "parse-equivalent" => Some(ConversionParityMode::ParseEquivalent),
        "diagnostic-equivalent" => Some(ConversionParityMode::DiagnosticEquivalent),
        _ => None,
    }
}

fn parse_manifest_bool(attrs: &BTreeMap<String, String>, attribute: &'static str) -> Option<bool> {
    let Some(value) = attrs.get(attribute).map(String::as_str).map(str::trim) else {
        return None;
    };
    match value {
        "" | "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_manifest_cost(attrs: &BTreeMap<String, String>) -> Option<u32> {
    let Some(value) = optional_manifest_attr(attrs, "cost") else {
        return None;
    };
    value.parse::<u32>().ok().filter(|cost| *cost >= 1)
}

fn optional_manifest_attr<'a>(
    attrs: &'a BTreeMap<String, String>,
    attribute: &str,
) -> Option<&'a str> {
    attrs
        .get(attribute)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_cem_document(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

fn collect_manifest_attrs(document: &CemDocument, node_id: AstNodeId) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = document.get(node_id) else {
        return attrs;
    };

    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attr_id)
        else {
            continue;
        };
        attrs.insert(
            expanded_name.local_name.clone(),
            value.clone().unwrap_or_default(),
        );
    }

    attrs
}

fn first_element_id_by_local_name(document: &CemDocument, local_name: &str) -> Option<AstNodeId> {
    document.iter().find_map(|node| {
        let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        else {
            return None;
        };
        (expanded_name.local_name == local_name).then_some(*node_id)
    })
}

fn element_child_ids_by_local_name(
    document: &CemDocument,
    node_id: AstNodeId,
    local_name: &str,
) -> Vec<AstNodeId> {
    let Some(CemAstNode::Element { children, .. }) = document.get(node_id) else {
        return Vec::new();
    };
    children
        .iter()
        .copied()
        .filter(|child_id| {
            matches!(
                document.get(*child_id),
                Some(CemAstNode::Element { expanded_name, .. })
                    if expanded_name.local_name == local_name
            )
        })
        .collect()
}

pub fn builtin_conversion_descriptors() -> Vec<ConversionDescriptor> {
    builtin_converter_package_schema_uris()
        .iter()
        .flat_map(|schema_uri| builtin_package_conversion_descriptors(schema_uri))
        .collect()
}

pub fn builtin_conversion_package_artifacts() -> Vec<ConversionPackageArtifactDescriptor> {
    builtin_converter_package_schema_uris()
        .iter()
        .flat_map(|schema_uri| builtin_package_conversion_package_artifacts(schema_uri))
        .collect()
}

fn builtin_package_conversion_descriptors(schema_uri: &str) -> Vec<ConversionDescriptor> {
    let package = load_builtin_schema_package(schema_uri)
        .expect("built-in converter package must have embedded sources");
    conversion_descriptors_from_schema_package(&package)
        .expect("built-in package converter metadata must be valid")
}

fn builtin_package_conversion_package_artifacts(
    schema_uri: &str,
) -> Vec<ConversionPackageArtifactDescriptor> {
    let package = load_builtin_schema_package(schema_uri)
        .expect("built-in converter package must have embedded sources");
    conversion_package_artifacts_from_schema_package(&package)
        .expect("built-in package artifact metadata must be valid")
}

fn builtin_converter_package_schema_uris() -> &'static [&'static str] {
    &[
        CEM_ML_SCHEMA_URI,
        HTML_SCHEMA_URI,
        XML_SCHEMA_URI,
        CEM_DOM_PROJECTION_SCHEMA_URI,
        CEM_AST_PROJECTION_SCHEMA_URI,
        CEM_EVENTS_PROJECTION_SCHEMA_URI,
        CEM_QL_SCHEMA_URI,
        JSON_VALUE_SCHEMA_URI,
        JSON_SCHEMA_SCHEMA_URI,
        CSV_SCHEMA_URI,
        YAML_SCHEMA_URI,
        MARKDOWN_SCHEMA_URI,
    ]
}

#[cfg(test)]
fn endpoint(content_type: &str, schema: &str) -> ConversionEndpoint {
    ConversionEndpoint::with_schema(content_type, schema)
}

#[cfg(test)]
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
        output_contract: ConversionOutputContractDescriptor::default(),
        parity_fixtures: Vec::new(),
        implicit: true,
        explicit_only: false,
        cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{EngineContext, TransformTemplateKind};
    use crate::schema::registry::{
        NamespaceClaim, SchemaContentType, SchemaDescriptor, AI_CONTEXT_JSON_CONTENT_TYPE,
        AI_CONTEXT_SCHEMA_URI, CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
        CEM_AST_PROJECTION_CONTENT_TYPE, CEM_AST_PROJECTION_SCHEMA_URI,
        CEM_DOM_JSON_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_CONTENT_TYPE,
        CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_CONTENT_TYPE,
        CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA_URI,
        CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, CSS_CONTENT_TYPE, CSS_SCHEMA_URI,
        CSV_CONTENT_TYPE, CSV_SCHEMA_URI, HTML_CONTENT_TYPE, JSON_CONTENT_TYPE,
        JSON_VALUE_SCHEMA_URI, MARKDOWN_CONTENT_TYPE, MARKDOWN_SCHEMA_URI,
        RELAX_NG_COMPACT_CONTENT_TYPE, RELAX_NG_SCHEMA_URI, XML_CONTENT_TYPE, YAML_CONTENT_TYPE,
        YAML_SCHEMA_URI,
    };
    use crate::transform_template::{
        TransformTemplateAdapter, TransformTemplateAdapterCapability,
        TransformTemplateAdapterRegistry, TransformTemplateColorOutputKind,
        TransformTemplateEncodedArtifact, TransformTemplateEncodedArtifactIdentity,
        TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_CONTEXT_MISMATCH_CODE,
        TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_DOUBLE_ENCODING_CODE,
    };

    fn identity(content_type: &str) -> FormatIdentity {
        FormatIdentity {
            content_type: Some(content_type.to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn execute_test_conversion_cem_tree_output_stage(
        stage: CemTreeCemtOutputStage,
        binding: &TransformTemplateEncodeBinding,
        subject: &Value,
    ) -> Result<(Value, ConversionOutputPipelineStageExecution), String> {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::new();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        execute_conversion_cem_tree_output_stage(&environment, stage, binding, subject)
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

    fn schema_package_attribute_values(attribute_name: &str) -> BTreeSet<String> {
        let source =
            builtin_schema_package_source("schema-package").expect("schema-package source");
        let document = parse_cem_document(source.schema_source);
        let schema_id = first_element_id_by_local_name(&document, "schema").expect("schema root");
        let attributes_id = element_child_ids_by_local_name(&document, schema_id, "attributes")
            .into_iter()
            .next()
            .expect("attributes section");

        for attribute_id in element_child_ids_by_local_name(&document, attributes_id, "attribute") {
            let attrs = collect_manifest_attrs(&document, attribute_id);
            if optional_manifest_attr(&attrs, "name") == Some(attribute_name) {
                return optional_manifest_attr(&attrs, "values")
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
            }
        }

        panic!("schema-package attribute `{attribute_name}` not declared");
    }

    fn schema_package_field_contract_when_values(contract_name: &str) -> BTreeSet<String> {
        let source =
            builtin_schema_package_source("schema-package").expect("schema-package source");
        let document = parse_cem_document(source.schema_source);
        let schema_id = first_element_id_by_local_name(&document, "schema").expect("schema root");
        let contracts_id = element_child_ids_by_local_name(&document, schema_id, "field-contracts")
            .into_iter()
            .next()
            .expect("field-contracts section");

        for contract_id in
            element_child_ids_by_local_name(&document, contracts_id, "field-contract")
        {
            let attrs = collect_manifest_attrs(&document, contract_id);
            if optional_manifest_attr(&attrs, "name") == Some(contract_name) {
                return optional_manifest_attr(&attrs, "when-values")
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
            }
        }

        panic!("schema-package field contract `{contract_name}` not declared");
    }

    fn string_set(values: &[&str]) -> BTreeSet<String> {
        values.iter().copied().map(str::to_owned).collect()
    }

    fn cemt_stage_required_metadata_labels(
        contract: CemtStageMetadataContract,
    ) -> BTreeSet<String> {
        contract
            .required_metadata_terms()
            .iter()
            .map(|(label, _)| (*label).to_owned())
            .collect()
    }

    fn accepted_manifest_values<T>(
        candidates: &[&str],
        parser: impl Fn(&str) -> Option<T>,
    ) -> BTreeSet<String> {
        candidates
            .iter()
            .copied()
            .filter(|candidate| parser(candidate).is_some())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn schema_package_manifest_enum_parsers_track_schema_declared_values() {
        assert_eq!(
            schema_package_attribute_values("implementation"),
            accepted_manifest_values(&["cemt", "rust"], parse_manifest_implementation)
        );
        assert_eq!(
            schema_package_attribute_values("readiness"),
            accepted_manifest_values(&["ready", "planned"], parse_manifest_readiness)
        );
        assert_eq!(
            schema_package_attribute_values("output-syntax"),
            accepted_manifest_values(
                &[
                    "html", "xml", "json", "yaml", "csv", "css", "markdown", "cemt", "text",
                    "binary", "opaque",
                ],
                parse_manifest_output_syntax,
            )
        );
        assert_eq!(
            schema_package_attribute_values("parity"),
            accepted_manifest_values(
                &[
                    "byte-exact",
                    "token-equivalent",
                    "parse-equivalent",
                    "diagnostic-equivalent",
                ],
                parse_manifest_parity_mode,
            )
        );

        assert!(parse_manifest_implementation("__schema-test-invalid__").is_none());
        assert!(parse_manifest_readiness("__schema-test-invalid__").is_none());
        assert!(parse_manifest_output_syntax("__schema-test-invalid__").is_none());
        assert!(parse_manifest_parity_mode("__schema-test-invalid__").is_none());
    }

    #[test]
    fn schema_package_artifact_stage_kind_groups_track_field_contracts() {
        let formatter_kinds = string_set(CemtStageMetadataContract::Formatter.artifact_kinds());
        let colorizer_kinds = string_set(CemtStageMetadataContract::Colorizer.artifact_kinds());
        let all_stage_kinds = formatter_kinds
            .union(&colorizer_kinds)
            .cloned()
            .collect::<BTreeSet<_>>();

        assert_eq!(
            schema_package_field_contract_when_values("artifact-formatter-layout"),
            formatter_kinds
        );
        assert_eq!(
            schema_package_field_contract_when_values("artifact-colorizer-layout"),
            colorizer_kinds
        );
        assert_eq!(
            schema_package_field_contract_when_values("artifact-stage-metadata"),
            all_stage_kinds
        );
    }

    #[test]
    fn cemt_artifact_stage_function_kind_mapping_tracks_schema_stage_groups() {
        for kind in schema_package_field_contract_when_values("artifact-formatter-layout") {
            assert_eq!(
                package_artifact_output_function_kind(&kind),
                Some(TransformTemplateOutputFunctionKind::Format),
                "formatter schema stage kind `{kind}` must map to CEMT format output functions"
            );
        }
        for kind in schema_package_field_contract_when_values("artifact-colorizer-layout") {
            assert_eq!(
                package_artifact_output_function_kind(&kind),
                Some(TransformTemplateOutputFunctionKind::Color),
                "colorizer schema stage kind `{kind}` must map to CEMT color output functions"
            );
        }
        assert_eq!(
            package_artifact_output_function_kind("__schema-test-invalid__"),
            None
        );
        assert_eq!(
            cemt_output_stage_helper_artifact_kind(TransformTemplateOutputFunctionKind::Format),
            Some(CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND)
        );
        assert_eq!(
            cemt_output_stage_helper_artifact_kind(TransformTemplateOutputFunctionKind::Color),
            Some(CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND)
        );
        assert_eq!(
            cemt_output_stage_helper_artifact_kind(TransformTemplateOutputFunctionKind::Encoding),
            None
        );
    }

    #[test]
    fn cemt_artifact_stage_metadata_term_contracts_are_operationally_explicit() {
        assert_eq!(
            cemt_stage_required_metadata_labels(CemtStageMetadataContract::Formatter),
            string_set(&[
                "formatterProfile",
                "formatNodes",
                "cem.format-tree",
                "format-marker",
                "format-decision",
                "formatterRole",
            ])
        );
        assert_eq!(
            cemt_stage_required_metadata_labels(CemtStageMetadataContract::Colorizer),
            string_set(&[
                "colored",
                "colorProfile",
                "colorNodes",
                "cem.color-tree",
                "color-marker",
                "color-decision",
                "colorizerRole",
            ])
        );
    }

    fn cemt_edge_with_output_contract(
        id: &str,
        to: ConversionEndpoint,
        output_contract: ConversionOutputContractDescriptor,
    ) -> ConversionDescriptor {
        ConversionDescriptor {
            id: id.to_owned(),
            package_id: "test-dom-projection".to_owned(),
            from: endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            to,
            implementation: ConversionImplementation::Cemt,
            readiness: ConversionReadiness::Ready,
            template: Some(ConversionTemplateDescriptor {
                path: "schema-packages/test/converters/dom-to-target.cemt".to_owned(),
                content_type: CEM_TRANSFORM_CONTENT_TYPE.to_owned(),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                entrypoint: Some("main".to_owned()),
            }),
            rust_symbol: None,
            rust_fallback: None,
            streamable: true,
            lossiness: Some("serialization".to_owned()),
            output_contract,
            parity_fixtures: Vec::new(),
            implicit: true,
            explicit_only: false,
            cost: 1,
        }
    }

    fn package_source(manifest_source: &'static str) -> BuiltinSchemaPackage {
        let mut descriptor = descriptor(
            CEM_DOM_PROJECTION_SCHEMA_URI,
            SchemaContentType::primary(CEM_DOM_PROJECTION_CONTENT_TYPE),
        );
        descriptor.package_id = "cem-dom-projection".to_owned();
        descriptor.source =
            "schema-packages/cem-dom-projection/v1/schema/cem-dom-projection.cem".to_owned();
        BuiltinSchemaPackage {
            descriptor,
            manifest_source,
            schema_source: "",
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
        assert_eq!(selection.descriptor.readiness, ConversionReadiness::Ready);

        let template = selection.descriptor.template.as_ref().unwrap();
        assert_eq!(
            template.path,
            "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
        );
        assert_eq!(template.content_type, CEM_TRANSFORM_CONTENT_TYPE);
        assert_eq!(template.schema.as_deref(), Some(CEM_TRANSFORM_SCHEMA_URI));
        assert_eq!(template.entrypoint.as_deref(), Some("main"));

        let fallback = selection.descriptor.rust_fallback.as_ref().unwrap();
        assert_eq!(fallback.rust_symbol, "HtmlExportConverter");
        assert!(fallback.reason.contains("executable CEMT adapter"));

        let rust_edge = registry
            .converter("cem-dom-projection-to-html-rust")
            .expect("rust fallback edge remains registered");
        assert_eq!(rust_edge.implementation, ConversionImplementation::Rust);
        assert_eq!(rust_edge.readiness, ConversionReadiness::Ready);
        assert_eq!(rust_edge.cost, 100);
    }

    #[test]
    fn builtin_dom_projection_cemt_assets_exist_and_are_ready() {
        let registry = ConversionRegistry::with_builtin_converters();

        for (id, expected_path, marker) in [
            (
                "cem-dom-projection-to-html-cemt",
                "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
                "DOM-to-HTML",
            ),
            (
                "cem-dom-projection-to-xml-cemt",
                "schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt",
                "DOM-to-XML",
            ),
        ] {
            let descriptor = registry.converter(id).expect("built-in converter");
            assert_eq!(descriptor.implementation, ConversionImplementation::Cemt);
            assert_eq!(descriptor.readiness, ConversionReadiness::Ready);

            let template = descriptor.template.as_ref().expect("CEMT template");
            assert_eq!(template.path, expected_path);
            assert_eq!(template.content_type, CEM_TRANSFORM_CONTENT_TYPE);
            assert_eq!(template.schema.as_deref(), Some(CEM_TRANSFORM_SCHEMA_URI));
            assert_eq!(template.entrypoint.as_deref(), Some("main"));

            let asset_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&template.path);
            let source = std::fs::read_to_string(&asset_path).unwrap_or_else(|err| {
                panic!(
                    "{marker} CEMT asset `{}` should be readable: {err}",
                    asset_path.display()
                )
            });
            assert!(source.starts_with("@doc cem-ml 1"));
            assert!(source.contains("@default transform"));
            assert!(source.contains(r#"{template @name="emit-node""#));
        }
    }

    #[test]
    fn builtin_package_manifest_declares_dom_projection_cemt_converters() {
        let package = load_builtin_schema_package(CEM_DOM_PROJECTION_SCHEMA_URI).unwrap();
        let descriptors = conversion_descriptors_from_schema_package(&package).unwrap();

        assert_eq!(descriptors.len(), 5);
        let html = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML converter descriptor");
        assert_eq!(html.package_id, "cem-dom-projection");
        assert_eq!(html.implementation, ConversionImplementation::Cemt);
        assert_eq!(html.readiness, ConversionReadiness::Ready);
        assert_eq!(html.from.content_type, CEM_DOM_PROJECTION_CONTENT_TYPE);
        assert_eq!(
            html.from.schema.as_deref(),
            Some(CEM_DOM_PROJECTION_SCHEMA_URI)
        );
        assert_eq!(html.to.content_type, HTML_CONTENT_TYPE);
        assert_eq!(html.to.schema.as_deref(), Some(HTML_SCHEMA_URI));
        assert_eq!(html.lossiness.as_deref(), Some("serialization"));
        assert_eq!(
            html.output_contract.output_syntax,
            Some(ConversionOutputSyntax::Html)
        );
        assert_eq!(
            html.output_contract.encoding_category.as_deref(),
            Some("html-document")
        );
        assert_eq!(
            html.output_contract.formatter_profile.as_deref(),
            Some("compact")
        );
        assert_eq!(
            html.output_contract.color_profile.as_deref(),
            Some("classes")
        );
        assert_eq!(
            html.output_contract.parity,
            Some(ConversionParityMode::ParseEquivalent)
        );
        assert!(html.streamable);
        assert!(html.implicit);
        assert!(!html.explicit_only);
        assert_eq!(html.cost, 100);
        assert_eq!(html.parity_fixtures.len(), 1);
        assert_eq!(html.parity_fixtures[0].id, "basic-dom");
        assert_eq!(
            html.parity_fixtures[0].path,
            "schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin"
        );
        assert_eq!(
            html.parity_fixtures[0].content_type.as_deref(),
            Some(CEM_DOM_PROJECTION_CONTENT_TYPE)
        );
        assert_eq!(
            html.parity_fixtures[0].schema.as_deref(),
            Some(CEM_DOM_PROJECTION_SCHEMA_URI)
        );
        assert!(html.parity_fixtures[0].expected_diagnostic_codes.is_empty());

        let template = html.template.as_ref().expect("HTML CEMT template");
        assert_eq!(
            template.path,
            "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
        );
        assert_eq!(template.content_type, CEM_TRANSFORM_CONTENT_TYPE);
        assert_eq!(template.schema.as_deref(), Some(CEM_TRANSFORM_SCHEMA_URI));
        assert_eq!(template.entrypoint.as_deref(), Some("main"));

        let fallback = html.rust_fallback.as_ref().expect("HTML Rust fallback");
        assert_eq!(fallback.rust_symbol, "HtmlExportConverter");
        assert!(fallback.reason.contains("executable CEMT adapter"));

        let xml = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "cem-dom-projection-to-xml-cemt")
            .expect("XML converter descriptor");
        assert_eq!(xml.to.content_type, XML_CONTENT_TYPE);
        assert_eq!(
            xml.output_contract.output_syntax,
            Some(ConversionOutputSyntax::Xml)
        );
        assert_eq!(
            xml.output_contract.encoding_category.as_deref(),
            Some("xml-document")
        );
        assert_eq!(
            xml.output_contract.parity,
            Some(ConversionParityMode::ParseEquivalent)
        );
        assert_eq!(xml.parity_fixtures.len(), 1);
        assert_eq!(xml.parity_fixtures[0].id, "basic-dom");
        assert_eq!(
            xml.parity_fixtures[0].path,
            "schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin"
        );
        let xml_template = xml.template.as_ref().expect("XML CEMT template");
        assert_eq!(
            xml_template.path,
            "schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt"
        );
        assert_eq!(xml_template.entrypoint.as_deref(), Some("main"));
        assert_eq!(
            xml.rust_fallback.as_ref().unwrap().rust_symbol,
            "XmlExportConverter"
        );

        let debug = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "cem-dom-projection-to-json-debug-rust")
            .expect("DOM JSON debug converter descriptor");
        assert_eq!(debug.implementation, ConversionImplementation::Rust);
        assert_eq!(
            debug.rust_symbol.as_deref(),
            Some("DomJsonDebugProjectionConverter")
        );
        assert_eq!(debug.to.content_type, CEM_DOM_JSON_PROJECTION_CONTENT_TYPE);
        assert_eq!(debug.lossiness.as_deref(), Some("debug-view"));
        assert_eq!(debug.cost, 150);
    }

    #[test]
    fn builtin_cemt_output_pipeline_template_must_be_embedded() {
        let package =
            load_builtin_schema_package(CEM_DOM_PROJECTION_SCHEMA_URI).expect("DOM package");
        let manifest = package.manifest_source.replacen(
            r#"@template="converters/dom-to-html.cemt""#,
            r#"@template="converters/missing-dom-to-html.cemt""#,
            1,
        );
        let package = BuiltinSchemaPackage {
            manifest_source: Box::leak(manifest.into_boxed_str()),
            ..package
        };

        let error = conversion_descriptors_from_schema_package(&package)
            .expect_err("built-in CEMT output pipeline template source is required");

        match error {
            ConversionManifestError::ConverterTemplateContract {
                converter_id,
                path,
                message,
                ..
            } => {
                assert_eq!(converter_id, "cem-dom-projection-to-html-cemt");
                assert_eq!(
                    path,
                    "schema-packages/cem-dom-projection/v1/converters/missing-dom-to-html.cemt"
                );
                assert!(message.contains("template source is not embedded"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cemt_output_pipeline_template_must_compile_as_supported_converter() {
        let descriptor = cemt_edge_with_output_contract(
            "dom-to-html-cemt",
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            ConversionOutputContractDescriptor {
                output_syntax: Some(ConversionOutputSyntax::Html),
                encoding_category: Some("html-document".to_owned()),
                formatter_profile: Some("compact".to_owned()),
                color_profile: Some("classes".to_owned()),
                parity: Some(ConversionParityMode::ParseEquivalent),
            },
        );
        let template = descriptor.template.as_ref().expect("template");
        let source = r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {template @name="main" | {text "not a supported DOM converter"}}
}
"#;

        let error = validate_conversion_descriptor_cemt_output_template_source(
            &descriptor,
            template,
            source,
        )
        .expect_err("unsupported CEMT converter template is rejected");

        match error {
            ConversionManifestError::ConverterTemplateContract {
                converter_id,
                path,
                message,
                ..
            } => {
                assert_eq!(converter_id, "dom-to-html-cemt");
                assert_eq!(path, template.path);
                assert!(message.contains("formatted CEM tree before the writer"));
                assert!(message.contains("supported DOM projection converter"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validated_schema_package_manifest_allows_external_cemt_template_paths() {
        let package =
            load_builtin_schema_package(CEM_DOM_PROJECTION_SCHEMA_URI).expect("DOM package");
        let manifest = package.manifest_source.replacen(
            r#"@template="converters/dom-to-html.cemt""#,
            r#"@template="converters/external-dom-to-html.cemt""#,
            1,
        );

        let descriptors = conversion_descriptors_from_validated_schema_package_manifest(
            "cem-dom-projection",
            &manifest,
            "schema-packages/cem-dom-projection/v1",
        )
        .expect("external validated package descriptors are not tied to embedded sources");

        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML CEMT descriptor");
        assert_eq!(
            descriptor.template.as_ref().unwrap().path,
            "schema-packages/cem-dom-projection/v1/converters/external-dom-to-html.cemt"
        );
    }

    #[test]
    fn package_manifest_accepts_yaml_output_syntax() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {converter
        @id="dom-to-yaml-cemt"
        @implementation="cemt"
        @template="converters/dom-to-yaml.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1"
        @template-entrypoint="main"
        @output-syntax="yaml"
        @encoding-category="yaml-document"
        @parity="parse-equivalent" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="application/yaml" @schema="https://cem.dev/ns/data/yaml/1"}
    }
}"#,
        );

        let descriptors =
            conversion_descriptors_from_schema_package(&package).expect("yaml output syntax loads");
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "dom-to-yaml-cemt")
            .expect("yaml converter descriptor");

        assert_eq!(
            descriptor.output_contract.output_syntax,
            Some(ConversionOutputSyntax::Yaml)
        );
        assert_eq!(
            descriptor.output_contract.encoding_category.as_deref(),
            Some("yaml-document")
        );
    }

    #[test]
    fn package_manifest_extraction_does_not_own_converter_required_fields() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {converter
        @implementation="rust"
        @rust-symbol="DomHtmlConverter" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
    {converter
        @id="missing-implementation" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
    {converter
        @id="invalid-implementation"
        @implementation="python" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
    {converter
        @id="missing-from"
        @implementation="rust"
        @rust-symbol="DomHtmlConverter" |
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
    {converter
        @id="missing-to"
        @implementation="rust"
        @rust-symbol="DomHtmlConverter" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
    }
    {converter
        @id="from-missing-content-type"
        @implementation="rust"
        @rust-symbol="DomHtmlConverter" |
        {from @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
    {converter
        @id="to-missing-content-type"
        @implementation="rust"
        @rust-symbol="DomHtmlConverter" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @schema="https://cem.dev/ns/data/html/1"}
    }
    {converter
        @id="valid-rust"
        @implementation="rust"
        @rust-symbol="DomHtmlConverter" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}"#,
        );

        let descriptors = conversion_descriptors_from_schema_package(&package)
            .expect("schema-owned validation reports missing converter fields before extraction");

        assert_eq!(descriptors.len(), 1);
        let descriptor = &descriptors[0];
        assert_eq!(descriptor.id, "valid-rust");
        assert_eq!(descriptor.implementation, ConversionImplementation::Rust);
        assert_eq!(
            descriptor.from.content_type,
            CEM_DOM_PROJECTION_CONTENT_TYPE
        );
        assert_eq!(
            descriptor.from.schema.as_deref(),
            Some(CEM_DOM_PROJECTION_SCHEMA_URI)
        );
        assert_eq!(descriptor.to.content_type, HTML_CONTENT_TYPE);
        assert_eq!(descriptor.to.schema.as_deref(), Some(HTML_SCHEMA_URI));
    }

    #[test]
    fn package_manifest_extraction_does_not_own_rust_symbol_requirement() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {converter
        @id="dom-to-html-rust"
        @implementation="rust" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}"#,
        );

        let descriptors = conversion_descriptors_from_schema_package(&package)
            .expect("schema-owned validation reports missing rust-symbol before extraction");
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "dom-to-html-rust")
            .expect("Rust descriptor");

        assert_eq!(descriptor.implementation, ConversionImplementation::Rust);
        assert_eq!(descriptor.rust_symbol, None);
    }

    #[test]
    fn package_manifest_extraction_does_not_own_cemt_template_identity_requirement() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {converter
        @id="dom-to-html-cemt"
        @implementation="cemt"
        @template="converters/dom-to-html.cemt" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}"#,
        );

        let descriptors = conversion_descriptors_from_schema_package(&package)
            .expect("schema-owned validation reports incomplete CEMT template identity");
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "dom-to-html-cemt")
            .expect("CEMT descriptor");

        assert_eq!(descriptor.implementation, ConversionImplementation::Cemt);
        assert_eq!(descriptor.template, None);
    }

    #[test]
    fn cemt_execution_keeps_missing_template_operational_guard() {
        let mut descriptor = cemt_edge_with_output_contract(
            "dom-to-html-cemt",
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            ConversionOutputContractDescriptor::default(),
        );
        descriptor.template = None;

        let template_adapters = TransformTemplateAdapterRegistry::new();
        let error = resolve_descriptor_execution(&descriptor, &template_adapters)
            .expect_err("CEMT execution still requires a template descriptor");

        assert_eq!(
            error,
            ConversionExecutionError::MissingTemplate {
                converter_id: "dom-to-html-cemt".to_owned()
            }
        );
    }

    #[test]
    fn package_manifest_extraction_does_not_own_converter_scalar_field_validation() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {converter
        @id="dom-to-html-rust"
        @implementation="rust"
        @rust-symbol="DomHtmlConverter"
        @readiness="later"
        @streamable="maybe"
        @explicit-only="maybe"
        @implicit="maybe"
        @cost=0
        @output-syntax="pdf"
        @parity="mostly-equal" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}"#,
        );

        let descriptors = conversion_descriptors_from_schema_package(&package).expect(
            "schema-owned validation reports invalid converter scalar fields before extraction",
        );
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "dom-to-html-rust")
            .expect("Rust descriptor");

        assert_eq!(descriptor.readiness, ConversionReadiness::Ready);
        assert!(!descriptor.streamable);
        assert!(!descriptor.explicit_only);
        assert!(descriptor.implicit);
        assert_eq!(descriptor.cost, 100);
        assert_eq!(descriptor.output_contract.output_syntax, None);
        assert_eq!(descriptor.output_contract.parity, None);
    }

    #[test]
    fn package_manifest_extraction_does_not_own_artifact_generated_boolean_validation() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {artifact
        @kind="support"
        @path="artifacts/generated.cemt"
        @generated="maybe"}
}"#,
        );

        let artifacts = conversion_package_artifacts_from_schema_package(&package)
            .expect("schema-owned validation reports invalid generated boolean before extraction");
        let artifact = artifacts
            .iter()
            .find(|artifact| artifact.path.ends_with("artifacts/generated.cemt"))
            .expect("artifact descriptor");

        assert!(!artifact.generated);
    }

    #[test]
    fn package_manifest_extraction_does_not_own_artifact_required_fields() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {artifact @path="artifacts/missing-kind.cemt"}
    {artifact @kind="support"}
    {artifact
        @kind="support"
        @path="artifacts/valid.cemt"
        @generated=true}
}"#,
        );

        let artifacts = conversion_package_artifacts_from_schema_package(&package)
            .expect("schema-owned validation reports missing artifact fields before extraction");

        assert_eq!(artifacts.len(), 1);
        let artifact = &artifacts[0];
        assert_eq!(artifact.kind, "support");
        assert_eq!(
            artifact.path,
            "schema-packages/cem-dom-projection/v1/artifacts/valid.cemt"
        );
        assert!(artifact.generated);
    }

    #[test]
    fn package_manifest_extraction_does_not_own_parity_fixture_required_fields() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {converter
        @id="dom-to-html-cemt"
        @implementation="cemt"
        @template="converters/dom-to-html.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
        {parity-fixture @path="examples/missing-id.dom.json"}
        {parity-fixture @id="missing-path"}
        {parity-fixture
            @id="valid"
            @path="examples/valid.dom.json"
            @content-type="application/vnd.cem.dom+json"
            @schema="https://cem.dev/ns/projection/dom/1"
            @expected-diagnostics="cem.projection.dom.json_shape"}
    }
}"#,
        );

        let descriptors = conversion_descriptors_from_schema_package(&package).expect(
            "schema-owned validation reports missing parity fixture fields before extraction",
        );
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "dom-to-html-cemt")
            .expect("CEMT descriptor");

        assert_eq!(descriptor.parity_fixtures.len(), 1);
        let fixture = &descriptor.parity_fixtures[0];
        assert_eq!(fixture.id, "valid");
        assert_eq!(
            fixture.path,
            "schema-packages/cem-dom-projection/v1/examples/valid.dom.json"
        );
        assert_eq!(
            fixture.content_type.as_deref(),
            Some(CEM_DOM_JSON_PROJECTION_CONTENT_TYPE)
        );
        assert_eq!(
            fixture.schema.as_deref(),
            Some(CEM_DOM_PROJECTION_SCHEMA_URI)
        );
        assert_eq!(
            fixture.expected_diagnostic_codes,
            vec!["cem.projection.dom.json_shape".to_owned()]
        );
    }

    #[test]
    fn package_manifest_declares_converter_parity_fixtures() {
        let package = package_source(
            r#"@doc cem-ml 1
{package @id="test-dom-projection" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/projection/dom/1" @source="schema/cem-dom-projection.cem"}
    {content-type @value="application/vnd.cem.dom+cem-bin" @primary=true}
    {converter
        @id="dom-to-html-cemt"
        @implementation="cemt"
        @template="converters/dom-to-html.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1"
        @template-entrypoint="main"
        @output-syntax="html"
        @encoding-category="html-document"
        @parity="parse-equivalent" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
        {parity-fixture
            @id="invalid-kind"
            @path="examples/invalid-kind.dom.json"
            @content-type="application/vnd.cem.dom+json"
            @schema="https://cem.dev/ns/projection/dom/1"
            @expected-diagnostics="cem.projection.dom.json_shape cem.converter.parity_drift"}
    }
}"#,
        );

        let descriptors = conversion_descriptors_from_schema_package(&package)
            .expect("parity fixture metadata loads");
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "dom-to-html-cemt")
            .expect("converter descriptor");

        assert_eq!(descriptor.parity_fixtures.len(), 1);
        let fixture = &descriptor.parity_fixtures[0];
        assert_eq!(fixture.id, "invalid-kind");
        assert_eq!(
            fixture.path,
            "schema-packages/cem-dom-projection/v1/examples/invalid-kind.dom.json"
        );
        assert_eq!(
            fixture.content_type.as_deref(),
            Some(CEM_DOM_JSON_PROJECTION_CONTENT_TYPE)
        );
        assert_eq!(
            fixture.schema.as_deref(),
            Some(CEM_DOM_PROJECTION_SCHEMA_URI)
        );
        assert_eq!(
            fixture.expected_diagnostic_codes,
            vec![
                "cem.projection.dom.json_shape".to_owned(),
                "cem.converter.parity_drift".to_owned()
            ]
        );
    }

    #[test]
    fn declared_converter_parity_fixtures_load_file_inputs() {
        let package = load_builtin_schema_package(CEM_DOM_PROJECTION_SCHEMA_URI).unwrap();
        let descriptors = conversion_descriptors_from_schema_package(&package).unwrap();
        let html = descriptors
            .iter()
            .find(|descriptor| descriptor.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML converter descriptor");

        let fixtures = load_conversion_parity_fixtures(html, env!("CARGO_MANIFEST_DIR"))
            .expect("declared parity fixture loads from package path");

        assert_eq!(fixtures.len(), 1);
        let fixture = &fixtures[0];
        assert_eq!(fixture.id, "basic-dom");
        assert!(fixture.expected_diagnostics.is_empty());
        assert!(fixture.expected_diagnostic_codes.is_empty());
        let input = fixture.input.as_object().expect("fixture input object");
        assert_eq!(
            input.get("path").and_then(Value::as_str),
            Some("schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin")
        );
        assert_eq!(
            input.get("contentType").and_then(Value::as_str),
            Some(CEM_DOM_PROJECTION_CONTENT_TYPE)
        );
        assert_eq!(
            input.get("schema").and_then(Value::as_str),
            Some(CEM_DOM_PROJECTION_SCHEMA_URI)
        );
        let bytes = input
            .get("bytes")
            .and_then(Value::as_array)
            .expect("byte array");
        let expected_bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin"),
        )
        .expect("read expected fixture");
        assert_eq!(bytes.len(), expected_bytes.len());
        assert!(!bytes.is_empty());
    }

    #[test]
    fn rust_dom_projection_parity_executor_renders_declared_fixture_outputs() {
        let registry = ConversionRegistry::with_builtin_converters();
        let html = registry
            .converter("cem-dom-projection-to-html-rust")
            .expect("HTML Rust converter");
        let xml = registry
            .converter("cem-dom-projection-to-xml-rust")
            .expect("XML Rust converter");
        let cemt_html = registry
            .converter("cem-dom-projection-to-html-cemt")
            .expect("HTML CEMT converter");
        let fixtures = load_conversion_parity_fixtures(cemt_html, env!("CARGO_MANIFEST_DIR"))
            .expect("declared parity fixture loads from package path");
        let fixture = fixtures.first().expect("declared fixture");
        let executor = RustDomProjectionParityFixtureExecutor;
        let expected =
            "<article id=\"welcome\"><h1>Welcome</h1><p>This is a minimal CEM-ML document.</p></article>";

        let html_execution = executor.execute_conversion_parity_fixture(html, fixture);
        assert!(
            html_execution.diagnostics.is_empty(),
            "{:?}",
            html_execution.diagnostics
        );
        assert_eq!(
            html_execution.output,
            Some(Value::String(expected.to_owned()))
        );

        let xml_execution = executor.execute_conversion_parity_fixture(xml, fixture);
        assert!(
            xml_execution.diagnostics.is_empty(),
            "{:?}",
            xml_execution.diagnostics
        );
        assert_eq!(
            xml_execution.output,
            Some(Value::String(expected.to_owned()))
        );
    }

    #[test]
    fn cemt_template_parity_executor_checks_declared_contract_fixture() {
        let registry = ConversionRegistry::with_builtin_converters();
        let (contracts, diagnostics) = registry.cemt_native_parity_contracts();
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let html_contract = contracts
            .iter()
            .find(|contract| contract.cemt.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML CEMT parity contract");
        let fixtures =
            load_conversion_parity_fixtures(html_contract.cemt, env!("CARGO_MANIFEST_DIR"))
                .expect("declared parity fixture loads from package path");
        let mut template_adapters = TransformTemplateAdapterRegistry::new();
        template_adapters.register(DomProjectionParityCemtAdapter);
        let executor =
            CemtTemplateParityFixtureExecutor::new(env!("CARGO_MANIFEST_DIR"), &template_adapters);

        let diagnostics = evaluate_conversion_parity_fixtures(html_contract, &fixtures, &executor);

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn dom_projection_cemt_adapter_builds_formatted_cem_tree_envelope() {
        let tree = conversion_dom_projection_parity_cem_tree_document(
            &serde_json::json!({
                "children": [{
                    "kind": "element",
                    "name": "article",
                    "children": [
                        {"kind": "text", "value": "Ready "},
                        {
                            "kind": "element",
                            "name": "strong",
                            "children": [{"kind": "text", "value": "now"}]
                        }
                    ]
                }]
            }),
            &ScopeConfig {
                cemt_formatter_profile: Some("acme.showcase.format-tree".to_owned()),
                ..ScopeConfig::default()
            },
        )
        .expect("DOM CEMT adapter tree");

        assert_eq!(tree["kind"], "cem-tree");
        assert_eq!(tree["contentType"], CEM_ML_CONTENT_TYPE);
        assert_eq!(tree["schema"], CEM_ML_SCHEMA_URI);
        assert_eq!(tree["category"], "cem-tree");
        assert_eq!(tree["mode"], "document");
        assert_eq!(tree["canonical"], true);
        assert_eq!(tree["formatterProfile"], "acme.showcase.format-tree");
        assert_eq!(tree["formatNodes"][0]["name"], "cem.format-tree");
        assert_eq!(tree["formatNodes"][1]["name"], "converter-cemt");
        assert_eq!(
            tree["formatNodes"][1]["formatterProfile"],
            "acme.showcase.format-tree"
        );
        assert_eq!(tree["nodes"][0]["sourceMap"], Value::Null);
        assert_eq!(tree["nodes"][0]["attributes"], Value::Array(vec![]));
        assert_eq!(
            tree["nodes"][0]["formatLayout"]["formatterRole"],
            "formatter.layout"
        );
        assert_eq!(
            tree["nodes"][0]["children"][1]["formatLayout"]["formatterRole"],
            "formatter.inline-emphasis"
        );
        assert_eq!(
            tree["nodes"][0]["children"][1]["children"][0]["sourceMap"],
            Value::Null
        );
    }

    #[test]
    fn declared_conversion_parity_contract_evaluator_runs_all_declared_fixtures() {
        let registry = ConversionRegistry::with_builtin_converters();
        let mut template_adapters = TransformTemplateAdapterRegistry::new();
        template_adapters.register(DomProjectionParityCemtAdapter);
        let executor =
            CemtTemplateParityFixtureExecutor::new(env!("CARGO_MANIFEST_DIR"), &template_adapters);

        let diagnostics = evaluate_declared_conversion_parity_contracts(
            &registry,
            env!("CARGO_MANIFEST_DIR"),
            &executor,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn declared_conversion_parity_contract_evaluator_reports_fixture_load_errors() {
        let registry = ConversionRegistry::with_builtin_converters();
        let missing_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("missing-parity-fixture-root");

        let diagnostics = evaluate_declared_conversion_parity_contracts(
            &registry,
            missing_root,
            &RustDomProjectionParityFixtureExecutor,
        );

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.code == CONVERSION_PARITY_FIXTURE_LOAD_CODE
                && diagnostic.severity == Severity::Error
                && diagnostic.node.as_deref() == Some("basic-dom")
                && diagnostic.message.contains("could not read")
        }));
    }

    #[test]
    fn cemt_template_parity_executor_reports_selector_only_adapter() {
        let registry = ConversionRegistry::with_builtin_converters();
        let cemt_html = registry
            .converter("cem-dom-projection-to-html-cemt")
            .expect("HTML CEMT converter");
        let fixtures = load_conversion_parity_fixtures(cemt_html, env!("CARGO_MANIFEST_DIR"))
            .expect("declared parity fixture loads from package path");
        let template_adapters = TransformTemplateAdapterRegistry::with_builtin_adapters();
        let executor =
            CemtTemplateParityFixtureExecutor::new(env!("CARGO_MANIFEST_DIR"), &template_adapters);

        let execution = executor.execute_conversion_parity_fixture(cemt_html, &fixtures[0]);

        assert!(execution.output.is_none());
        assert_eq!(execution.diagnostics.len(), 1);
        assert_eq!(
            execution.diagnostics[0].code,
            CONVERSION_PARITY_FIXTURE_EXECUTION_CODE
        );
        assert!(execution.diagnostics[0].message.contains("selector-only"));
    }

    #[test]
    fn builtin_registry_declares_cemt_native_parity_contracts() {
        let registry = ConversionRegistry::with_builtin_converters();
        let (contracts, diagnostics) = registry.cemt_native_parity_contracts();

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(contracts.len(), 2);

        let html = contracts
            .iter()
            .find(|contract| contract.cemt.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML CEMT parity contract");
        assert_eq!(html.native.id, "cem-dom-projection-to-html-rust");
        assert_eq!(html.mode, ConversionParityMode::ParseEquivalent);

        let xml = contracts
            .iter()
            .find(|contract| contract.cemt.id == "cem-dom-projection-to-xml-cemt")
            .expect("XML CEMT parity contract");
        assert_eq!(xml.native.id, "cem-dom-projection-to-xml-rust");
        assert_eq!(xml.mode, ConversionParityMode::ParseEquivalent);
    }

    #[test]
    fn parity_contracts_report_missing_mode_and_native_pair() {
        let mut registry = ConversionRegistry::new();
        registry
            .register(ConversionDescriptor {
                id: "dom-to-html-cemt-missing-mode".to_owned(),
                package_id: "test-dom-projection".to_owned(),
                from: endpoint(
                    CEM_DOM_PROJECTION_CONTENT_TYPE,
                    CEM_DOM_PROJECTION_SCHEMA_URI,
                ),
                to: endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                implementation: ConversionImplementation::Cemt,
                readiness: ConversionReadiness::Ready,
                template: Some(ConversionTemplateDescriptor {
                    path: "schema-packages/test/converters/dom-to-html.cemt".to_owned(),
                    content_type: CEM_TRANSFORM_CONTENT_TYPE.to_owned(),
                    schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                    entrypoint: Some("main".to_owned()),
                }),
                rust_symbol: None,
                rust_fallback: Some(ConversionRustFallbackDescriptor {
                    rust_symbol: "MissingModeHtmlConverter".to_owned(),
                    reason: "test fallback".to_owned(),
                }),
                streamable: true,
                lossiness: Some("serialization".to_owned()),
                output_contract: ConversionOutputContractDescriptor::default(),
                parity_fixtures: Vec::new(),
                implicit: true,
                explicit_only: false,
                cost: 1,
            })
            .unwrap();
        registry
            .register(ConversionDescriptor {
                id: "dom-to-xml-cemt-missing-native".to_owned(),
                package_id: "test-dom-projection".to_owned(),
                from: endpoint(
                    CEM_DOM_PROJECTION_CONTENT_TYPE,
                    CEM_DOM_PROJECTION_SCHEMA_URI,
                ),
                to: endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
                implementation: ConversionImplementation::Cemt,
                readiness: ConversionReadiness::Ready,
                template: Some(ConversionTemplateDescriptor {
                    path: "schema-packages/test/converters/dom-to-xml.cemt".to_owned(),
                    content_type: CEM_TRANSFORM_CONTENT_TYPE.to_owned(),
                    schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                    entrypoint: Some("main".to_owned()),
                }),
                rust_symbol: None,
                rust_fallback: Some(ConversionRustFallbackDescriptor {
                    rust_symbol: "MissingNativeXmlConverter".to_owned(),
                    reason: "test fallback".to_owned(),
                }),
                streamable: true,
                lossiness: Some("serialization".to_owned()),
                output_contract: ConversionOutputContractDescriptor {
                    parity: Some(ConversionParityMode::ByteExact),
                    ..ConversionOutputContractDescriptor::default()
                },
                parity_fixtures: Vec::new(),
                implicit: true,
                explicit_only: false,
                cost: 1,
            })
            .unwrap();

        let (contracts, diagnostics) = registry.cemt_native_parity_contracts();

        assert!(contracts.is_empty());
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CONVERSION_PARITY_MODE_MISSING_CODE));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CONVERSION_PARITY_NATIVE_PAIR_MISSING_CODE));
    }

    #[test]
    fn parity_contracts_report_native_schema_output_without_cemt_pair() {
        let mut registry = ConversionRegistry::new();
        registry
            .register(rust_edge(
                "dom-to-html-rust-orphan",
                "test-dom-projection",
                endpoint(
                    CEM_DOM_PROJECTION_CONTENT_TYPE,
                    CEM_DOM_PROJECTION_SCHEMA_URI,
                ),
                endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                "OrphanHtmlExportConverter",
                "serialization",
                1,
            ))
            .unwrap();
        registry
            .register(rust_edge(
                "html-to-xml-rust-content-conversion",
                "test-html",
                endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
                "HtmlToXmlContentConverter",
                "serialization",
                1,
            ))
            .unwrap();

        let (contracts, diagnostics) = registry.cemt_native_parity_contracts();

        assert!(contracts.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            CONVERSION_PARITY_CEMT_PAIR_MISSING_CODE
        );
        assert!(diagnostics[0].message.contains("dom-to-html-rust-orphan"));
        assert!(!diagnostics[0]
            .message
            .contains("html-to-xml-rust-content-conversion"));
    }

    #[test]
    fn parity_output_comparison_reports_drift() {
        let registry = ConversionRegistry::with_builtin_converters();
        let (contracts, diagnostics) = registry.cemt_native_parity_contracts();
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let html = contracts
            .iter()
            .find(|contract| contract.cemt.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML CEMT parity contract");
        let xml = contracts
            .iter()
            .find(|contract| contract.cemt.id == "cem-dom-projection-to-xml-cemt")
            .expect("XML CEMT parity contract");

        assert!(compare_conversion_parity_outputs(
            html,
            &Value::String("<main>ok</main>".to_owned()),
            &Value::String("<main>ok</main>".to_owned()),
        )
        .is_none());
        assert!(compare_conversion_parity_outputs(
            html,
            &Value::String(r#"<main><p id="x" class="lead">ok</p></main>"#.to_owned()),
            &Value::String(
                r#"<main>
  <p class="lead" id="x">ok</p>
</main>"#
                    .to_owned()
            ),
        )
        .is_none());
        assert!(compare_conversion_parity_outputs(
            xml,
            &Value::String(r#"<root><p><![CDATA[Hi <all>]]></p></root>"#.to_owned()),
            &Value::String(r#"<root><p>Hi &lt;all&gt;</p></root>"#.to_owned()),
        )
        .is_none());

        let drift = compare_conversion_parity_outputs(
            html,
            &Value::String("<main>cemt</main>".to_owned()),
            &Value::String("<main>native</main>".to_owned()),
        )
        .expect("different outputs produce drift diagnostic");
        assert_eq!(drift.code, CONVERSION_PARITY_DRIFT_CODE);
        assert_eq!(drift.severity, Severity::Error);
        assert!(drift.message.contains("parse-equivalent"));
    }

    #[test]
    fn token_equivalent_parity_compares_token_projection() {
        let html_cemt = cemt_edge_with_output_contract(
            "dom-to-html-token-cemt",
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            ConversionOutputContractDescriptor {
                output_syntax: Some(ConversionOutputSyntax::Html),
                parity: Some(ConversionParityMode::TokenEquivalent),
                ..ConversionOutputContractDescriptor::default()
            },
        );
        let html_native = rust_edge(
            "dom-to-html-token-rust",
            "test-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            "HtmlTokenExportConverter",
            "serialization",
            1,
        );
        let html_contract = ConversionParityContract {
            cemt: &html_cemt,
            native: &html_native,
            mode: ConversionParityMode::TokenEquivalent,
        };

        assert!(compare_conversion_parity_outputs(
            &html_contract,
            &Value::String(r#"<main><p id="x">ok</p></main>"#.to_owned()),
            &Value::String(r#"<main><p id="x">ok</p></main>"#.to_owned()),
        )
        .is_none());

        let attr_order_drift = compare_conversion_parity_outputs(
            &html_contract,
            &Value::String(r#"<main><p id="x" class="lead">ok</p></main>"#.to_owned()),
            &Value::String(r#"<main><p class="lead" id="x">ok</p></main>"#.to_owned()),
        )
        .expect("token-equivalent parity preserves token order");
        assert_eq!(attr_order_drift.code, CONVERSION_PARITY_DRIFT_CODE);
        assert_eq!(attr_order_drift.severity, Severity::Error);
        assert!(attr_order_drift.message.contains("token-equivalent"));

        let xml_cemt = cemt_edge_with_output_contract(
            "dom-to-xml-token-cemt",
            endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
            ConversionOutputContractDescriptor {
                output_syntax: Some(ConversionOutputSyntax::Xml),
                parity: Some(ConversionParityMode::TokenEquivalent),
                ..ConversionOutputContractDescriptor::default()
            },
        );
        let xml_native = rust_edge(
            "dom-to-xml-token-rust",
            "test-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
            "XmlTokenExportConverter",
            "serialization",
            1,
        );
        let xml_contract = ConversionParityContract {
            cemt: &xml_cemt,
            native: &xml_native,
            mode: ConversionParityMode::TokenEquivalent,
        };

        let cdata_drift = compare_conversion_parity_outputs(
            &xml_contract,
            &Value::String(r#"<root><p><![CDATA[Hi <all>]]></p></root>"#.to_owned()),
            &Value::String(r#"<root><p>Hi &lt;all&gt;</p></root>"#.to_owned()),
        )
        .expect("token-equivalent parity preserves token kind");
        assert_eq!(cdata_drift.code, CONVERSION_PARITY_DRIFT_CODE);
        assert!(cdata_drift.message.contains("token-equivalent"));
    }

    #[test]
    fn diagnostic_equivalent_parity_compares_diagnostic_projection() {
        let cemt = cemt_edge_with_output_contract(
            "dom-diagnostics-cemt",
            endpoint(JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI),
            ConversionOutputContractDescriptor {
                output_syntax: Some(ConversionOutputSyntax::Json),
                parity: Some(ConversionParityMode::DiagnosticEquivalent),
                ..ConversionOutputContractDescriptor::default()
            },
        );
        let native = rust_edge(
            "dom-diagnostics-rust",
            "test-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI),
            "DiagnosticExportConverter",
            "diagnostics",
            1,
        );
        let contract = ConversionParityContract {
            cemt: &cemt,
            native: &native,
            mode: ConversionParityMode::DiagnosticEquivalent,
        };

        let cemt_output = serde_json::json!({
            "diagnostics": [
                {
                    "code": "cem.test.alpha",
                    "severity": "warning",
                    "message": "CEMT warning wording",
                    "uri": "file:///input.cem",
                    "line": 2,
                    "column": 5,
                    "byteOffset": 17,
                    "node": "node-1",
                    "sourceMap": {
                        "segments": [
                            {
                                "uri": "file:///input.cem",
                                "line": 2,
                                "column": 5
                            }
                        ]
                    }
                },
                {
                    "code": "cem.test.beta",
                    "severity": "error",
                    "message": "CEMT error wording"
                }
            ]
        });
        let native_output = serde_json::json!([
            {
                "code": "cem.test.beta",
                "severity": "error",
                "message": "Native error wording"
            },
            {
                "code": "cem.test.alpha",
                "severity": "warning",
                "message": "Native warning wording",
                "uri": "file:///input.cem",
                "line": 2,
                "column": 5,
                "byteOffset": 17,
                "node": "node-1",
                "sourceMap": {
                    "segments": [
                        {
                            "column": 5,
                            "line": 2,
                            "uri": "file:///input.cem"
                        }
                    ]
                }
            }
        ]);

        assert!(
            compare_conversion_parity_outputs(&contract, &cemt_output, &native_output).is_none()
        );

        let native_location_drift = serde_json::json!({
            "diagnostics": [
                {
                    "code": "cem.test.alpha",
                    "severity": "warning",
                    "message": "Native warning wording",
                    "uri": "file:///input.cem",
                    "line": 3,
                    "column": 5,
                    "byteOffset": 17,
                    "node": "node-1",
                    "sourceMap": {
                        "segments": [
                            {
                                "uri": "file:///input.cem",
                                "line": 2,
                                "column": 5
                            }
                        ]
                    }
                },
                {
                    "code": "cem.test.beta",
                    "severity": "error",
                    "message": "Native error wording"
                }
            ]
        });
        let drift =
            compare_conversion_parity_outputs(&contract, &cemt_output, &native_location_drift)
                .expect("diagnostic location drift is reported");
        assert_eq!(drift.code, CONVERSION_PARITY_DRIFT_CODE);
        assert_eq!(drift.severity, Severity::Error);
        assert!(drift.message.contains("diagnostic-equivalent"));
    }

    #[derive(Default)]
    struct TestConversionParityFixtureExecutor {
        outputs: std::collections::BTreeMap<(String, String), ConversionParityFixtureExecution>,
        calls: std::cell::RefCell<Vec<(String, String, Value)>>,
    }

    impl TestConversionParityFixtureExecutor {
        fn insert(
            &mut self,
            converter_id: &str,
            fixture_id: &str,
            execution: ConversionParityFixtureExecution,
        ) {
            self.outputs
                .insert((converter_id.to_owned(), fixture_id.to_owned()), execution);
        }
    }

    impl ConversionParityFixtureExecutor for TestConversionParityFixtureExecutor {
        fn execute_conversion_parity_fixture(
            &self,
            descriptor: &ConversionDescriptor,
            fixture: &ConversionParityFixture,
        ) -> ConversionParityFixtureExecution {
            self.calls.borrow_mut().push((
                descriptor.id.clone(),
                fixture.id.clone(),
                fixture.input.clone(),
            ));
            self.outputs
                .get(&(descriptor.id.clone(), fixture.id.clone()))
                .unwrap_or_else(|| {
                    panic!(
                        "missing fixture execution for converter `{}` fixture `{}`",
                        descriptor.id, fixture.id
                    )
                })
                .clone()
        }
    }

    fn conversion_parity_test_diagnostic(
        code: &str,
        severity: Severity,
        line: Option<u32>,
        message: &str,
    ) -> Diagnostic {
        Diagnostic {
            code: code.to_owned(),
            severity,
            line,
            message: message.to_owned(),
            details: None,
            ..Diagnostic::default()
        }
    }

    #[test]
    fn parity_fixture_execution_uses_shared_inputs_and_reports_output_drift() {
        let cemt = cemt_edge_with_output_contract(
            "fixture-html-cemt",
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            ConversionOutputContractDescriptor {
                output_syntax: Some(ConversionOutputSyntax::Html),
                parity: Some(ConversionParityMode::ByteExact),
                ..ConversionOutputContractDescriptor::default()
            },
        );
        let native = rust_edge(
            "fixture-html-rust",
            "test-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
            "HtmlFixtureExportConverter",
            "serialization",
            1,
        );
        let contract = ConversionParityContract {
            cemt: &cemt,
            native: &native,
            mode: ConversionParityMode::ByteExact,
        };
        let fixtures = vec![
            ConversionParityFixture {
                id: "matching".to_owned(),
                input: serde_json::json!({ "case": "matching" }),
                expected_diagnostics: Vec::new(),
                expected_diagnostic_codes: Vec::new(),
            },
            ConversionParityFixture {
                id: "output-drift".to_owned(),
                input: serde_json::json!({ "case": "output-drift" }),
                expected_diagnostics: Vec::new(),
                expected_diagnostic_codes: Vec::new(),
            },
        ];
        let mut executor = TestConversionParityFixtureExecutor::default();
        executor.insert(
            "fixture-html-cemt",
            "matching",
            ConversionParityFixtureExecution {
                output: Some(Value::String("<main>ok</main>".to_owned())),
                diagnostics: Vec::new(),
            },
        );
        executor.insert(
            "fixture-html-rust",
            "matching",
            ConversionParityFixtureExecution {
                output: Some(Value::String("<main>ok</main>".to_owned())),
                diagnostics: Vec::new(),
            },
        );
        executor.insert(
            "fixture-html-cemt",
            "output-drift",
            ConversionParityFixtureExecution {
                output: Some(Value::String("<main>cemt</main>".to_owned())),
                diagnostics: Vec::new(),
            },
        );
        executor.insert(
            "fixture-html-rust",
            "output-drift",
            ConversionParityFixtureExecution {
                output: Some(Value::String("<main>native</main>".to_owned())),
                diagnostics: Vec::new(),
            },
        );

        let diagnostics = evaluate_conversion_parity_fixtures(&contract, &fixtures, &executor);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, CONVERSION_PARITY_DRIFT_CODE);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(diagnostics[0].node.as_deref(), Some("output-drift"));
        assert!(diagnostics[0].message.contains("fixture `output-drift`"));
        assert!(diagnostics[0].message.contains("byte-exact"));
        assert!(diagnostics[0].message.contains("output"));

        let calls = executor.calls.borrow();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].0, "fixture-html-cemt");
        assert_eq!(calls[1].0, "fixture-html-rust");
        assert_eq!(calls[0].1, "matching");
        assert_eq!(calls[1].1, "matching");
        assert_eq!(calls[0].2, serde_json::json!({ "case": "matching" }));
        assert_eq!(calls[1].2, serde_json::json!({ "case": "matching" }));
        assert_eq!(calls[2].0, "fixture-html-cemt");
        assert_eq!(calls[3].0, "fixture-html-rust");
        assert_eq!(calls[2].1, "output-drift");
        assert_eq!(calls[3].1, "output-drift");
        assert_eq!(calls[2].2, serde_json::json!({ "case": "output-drift" }));
        assert_eq!(calls[3].2, serde_json::json!({ "case": "output-drift" }));
    }

    #[test]
    fn parity_fixture_execution_compares_expected_diagnostics() {
        let cemt = cemt_edge_with_output_contract(
            "fixture-diagnostics-cemt",
            endpoint(JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI),
            ConversionOutputContractDescriptor {
                output_syntax: Some(ConversionOutputSyntax::Json),
                parity: Some(ConversionParityMode::ByteExact),
                ..ConversionOutputContractDescriptor::default()
            },
        );
        let native = rust_edge(
            "fixture-diagnostics-rust",
            "test-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI),
            "DiagnosticFixtureExportConverter",
            "diagnostics",
            1,
        );
        let contract = ConversionParityContract {
            cemt: &cemt,
            native: &native,
            mode: ConversionParityMode::ByteExact,
        };
        let expected = conversion_parity_test_diagnostic(
            "cem.fixture.warning",
            Severity::Warning,
            Some(2),
            "expected wording",
        );
        let fixtures = vec![
            ConversionParityFixture {
                id: "diagnostic-equivalent".to_owned(),
                input: serde_json::json!({ "case": "diagnostic-equivalent" }),
                expected_diagnostics: vec![expected.clone()],
                expected_diagnostic_codes: Vec::new(),
            },
            ConversionParityFixture {
                id: "diagnostic-drift".to_owned(),
                input: serde_json::json!({ "case": "diagnostic-drift" }),
                expected_diagnostics: vec![expected.clone()],
                expected_diagnostic_codes: Vec::new(),
            },
        ];
        let mut executor = TestConversionParityFixtureExecutor::default();
        for converter_id in ["fixture-diagnostics-cemt", "fixture-diagnostics-rust"] {
            executor.insert(
                converter_id,
                "diagnostic-equivalent",
                ConversionParityFixtureExecution {
                    output: Some(serde_json::json!({ "status": "ok" })),
                    diagnostics: vec![conversion_parity_test_diagnostic(
                        "cem.fixture.warning",
                        Severity::Warning,
                        Some(2),
                        if converter_id.ends_with("cemt") {
                            "CEMT wording"
                        } else {
                            "native wording"
                        },
                    )],
                },
            );
        }
        executor.insert(
            "fixture-diagnostics-cemt",
            "diagnostic-drift",
            ConversionParityFixtureExecution {
                output: Some(serde_json::json!({ "status": "ok" })),
                diagnostics: vec![conversion_parity_test_diagnostic(
                    "cem.fixture.warning",
                    Severity::Warning,
                    Some(2),
                    "CEMT wording",
                )],
            },
        );
        executor.insert(
            "fixture-diagnostics-rust",
            "diagnostic-drift",
            ConversionParityFixtureExecution {
                output: Some(serde_json::json!({ "status": "ok" })),
                diagnostics: vec![conversion_parity_test_diagnostic(
                    "cem.fixture.warning",
                    Severity::Error,
                    Some(2),
                    "native wording",
                )],
            },
        );

        let diagnostics = evaluate_conversion_parity_fixtures(&contract, &fixtures, &executor);

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.code == CONVERSION_PARITY_DRIFT_CODE
                && diagnostic.node.as_deref() == Some("diagnostic-drift")
                && diagnostic.message.contains("diagnostics")
        }));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("diagnostics differ")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("native.diagnostics")));
    }

    #[test]
    fn parity_fixture_execution_accepts_expected_diagnostic_code_projection() {
        let cemt = cemt_edge_with_output_contract(
            "fixture-code-projection-cemt",
            endpoint(JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI),
            ConversionOutputContractDescriptor {
                output_syntax: Some(ConversionOutputSyntax::Json),
                parity: Some(ConversionParityMode::ByteExact),
                ..ConversionOutputContractDescriptor::default()
            },
        );
        let native = rust_edge(
            "fixture-code-projection-rust",
            "test-dom-projection",
            endpoint(
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
            ),
            endpoint(JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI),
            "DiagnosticCodeProjectionConverter",
            "diagnostics",
            1,
        );
        let contract = ConversionParityContract {
            cemt: &cemt,
            native: &native,
            mode: ConversionParityMode::ByteExact,
        };
        let fixtures = vec![
            ConversionParityFixture {
                id: "code-equivalent".to_owned(),
                input: serde_json::json!({ "case": "code-equivalent" }),
                expected_diagnostics: Vec::new(),
                expected_diagnostic_codes: vec!["cem.fixture.warning".to_owned()],
            },
            ConversionParityFixture {
                id: "code-drift".to_owned(),
                input: serde_json::json!({ "case": "code-drift" }),
                expected_diagnostics: Vec::new(),
                expected_diagnostic_codes: vec!["cem.fixture.warning".to_owned()],
            },
        ];
        let mut executor = TestConversionParityFixtureExecutor::default();
        for converter_id in [
            "fixture-code-projection-cemt",
            "fixture-code-projection-rust",
        ] {
            executor.insert(
                converter_id,
                "code-equivalent",
                ConversionParityFixtureExecution {
                    output: Some(serde_json::json!({ "status": "ok" })),
                    diagnostics: vec![conversion_parity_test_diagnostic(
                        "cem.fixture.warning",
                        Severity::Warning,
                        None,
                        if converter_id.ends_with("cemt") {
                            "CEMT wording"
                        } else {
                            "native wording"
                        },
                    )],
                },
            );
        }
        executor.insert(
            "fixture-code-projection-cemt",
            "code-drift",
            ConversionParityFixtureExecution {
                output: Some(serde_json::json!({ "status": "ok" })),
                diagnostics: vec![conversion_parity_test_diagnostic(
                    "cem.fixture.warning",
                    Severity::Warning,
                    None,
                    "CEMT wording",
                )],
            },
        );
        executor.insert(
            "fixture-code-projection-rust",
            "code-drift",
            ConversionParityFixtureExecution {
                output: Some(serde_json::json!({ "status": "ok" })),
                diagnostics: vec![conversion_parity_test_diagnostic(
                    "cem.fixture.other",
                    Severity::Warning,
                    None,
                    "native wording",
                )],
            },
        );

        let diagnostics = evaluate_conversion_parity_fixtures(&contract, &fixtures, &executor);

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.code == CONVERSION_PARITY_DRIFT_CODE
                && diagnostic.node.as_deref() == Some("code-drift")
                && diagnostic.message.contains("diagnostics")
        }));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("diagnostic streams differ")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("native.diagnostics")));
    }

    #[test]
    fn builtin_registry_declares_cemt_output_safety_contracts() {
        let registry = ConversionRegistry::with_builtin_converters();
        let (contracts, diagnostics) = registry.cemt_output_safety_contracts();

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(contracts.len(), 2);

        let html = contracts
            .iter()
            .find(|contract| contract.descriptor.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML output safety contract");
        assert_eq!(html.target.content_type, HTML_CONTENT_TYPE);
        assert_eq!(html.target.schema, HTML_SCHEMA_URI);
        assert_eq!(html.target.category, "html-document");
        assert_eq!(
            html.syntax_rules.syntax,
            TransformTemplateTargetSyntaxKind::Html
        );
        assert_eq!(html.produces, TransformTemplateOutputProducedKind::Text);
        assert_eq!(
            html.insertion_context.category.as_deref(),
            Some("html-document")
        );
        assert_eq!(
            html.insertion_context.formatter_profile.as_deref(),
            Some("compact")
        );
        assert_eq!(
            html.insertion_context.color_profile.as_deref(),
            Some("classes")
        );
        assert_eq!(
            html.insertion_context.mode,
            Some(TransformTemplateEncodedArtifactMode::Document)
        );
        assert_eq!(html.insertion_context.canonical, Some(true));
        assert_eq!(
            html.options.source_map_policy,
            TransformTemplateSourceMapPolicy::Generated
        );
        assert_eq!(
            html.insertion_context.source_map_policy,
            Some(TransformTemplateSourceMapPolicy::Generated)
        );
        assert_eq!(
            html.pipeline.stages,
            vec![
                ConversionOutputPipelineStage::Transform,
                ConversionOutputPipelineStage::Format,
                ConversionOutputPipelineStage::Color,
                ConversionOutputPipelineStage::Writer,
            ]
        );
        assert_eq!(html.pipeline.cemt_target.content_type, CEM_ML_CONTENT_TYPE);
        assert_eq!(html.pipeline.cemt_target.schema, CEM_ML_SCHEMA_URI);
        assert_eq!(html.pipeline.cemt_target.category, "cem-tree");
        assert_eq!(
            html.pipeline.cemt_produces,
            TransformTemplateOutputProducedKind::CemTree
        );
        assert_eq!(html.pipeline.cemt_options.formatter.as_deref(), None);
        assert_eq!(
            html.pipeline.cemt_options.formatter_profile.as_deref(),
            Some("compact")
        );
        assert_eq!(html.pipeline.cemt_options.colorizer.as_deref(), None);
        assert_eq!(
            html.pipeline.cemt_options.color_profile.as_deref(),
            Some("classes")
        );
        assert_eq!(
            html.pipeline.cemt_insertion_context.produces,
            Some(TransformTemplateOutputProducedKind::CemTree)
        );
        assert_eq!(
            html.pipeline
                .cemt_insertion_context
                .formatter_profile
                .as_deref(),
            Some("compact")
        );
        assert_eq!(
            html.pipeline
                .cemt_insertion_context
                .color_profile
                .as_deref(),
            Some("classes")
        );
        assert_eq!(
            html.pipeline.writer_insertion_context,
            html.insertion_context
        );
        assert_eq!(
            html.pipeline.writer_produces,
            TransformTemplateOutputProducedKind::Text
        );
        assert!(html.options.charset.is_none());
        assert_eq!(
            html.syntax_rules
                .writer_boundaries
                .default_charset
                .as_deref(),
            Some("utf-8")
        );
        let color = html
            .color_output_profile
            .as_ref()
            .expect("HTML color profile");
        assert_eq!(color.output, TransformTemplateColorOutputKind::Html);
        assert!(color.supports_role("diagnostic.error"));
        color.validate().expect("HTML color profile is safe");

        let xml = contracts
            .iter()
            .find(|contract| contract.descriptor.id == "cem-dom-projection-to-xml-cemt")
            .expect("XML output safety contract");
        assert_eq!(xml.target.category, "xml-document");
        assert_eq!(
            xml.syntax_rules.syntax,
            TransformTemplateTargetSyntaxKind::Xml
        );
        assert_eq!(xml.insertion_context.color_profile, None);
        assert_eq!(xml.pipeline.cemt_options.colorizer.as_deref(), None);
        assert_eq!(
            xml.pipeline.cemt_options.color_profile.as_deref(),
            Some("none")
        );
        assert_eq!(
            xml.pipeline.cemt_insertion_context.color_profile.as_deref(),
            Some("none")
        );
        assert_eq!(
            xml.syntax_rules
                .writer_boundaries
                .default_charset
                .as_deref(),
            Some("utf-8")
        );
    }

    #[test]
    fn encoding_category_examples_cover_proposal_content_type_families() {
        struct FamilyCase {
            family: &'static str,
            content_type: &'static str,
            schema: &'static str,
            category: &'static str,
            output_syntax: ConversionOutputSyntax,
            expected_syntax: TransformTemplateTargetSyntaxKind,
            expected_produces: TransformTemplateOutputProducedKind,
        }

        let cases = [
            FamilyCase {
                family: "CEM-ML syntax",
                content_type: CEM_ML_CONTENT_TYPE,
                schema: CEM_ML_SCHEMA_URI,
                category: "cem-document",
                output_syntax: ConversionOutputSyntax::Cemt,
                expected_syntax: TransformTemplateTargetSyntaxKind::Cemt,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "CEMT source",
                content_type: CEM_TRANSFORM_CONTENT_TYPE,
                schema: CEM_TRANSFORM_SCHEMA_URI,
                category: "cemt-module",
                output_syntax: ConversionOutputSyntax::Cemt,
                expected_syntax: TransformTemplateTargetSyntaxKind::Cemt,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "XML family",
                content_type: XML_CONTENT_TYPE,
                schema: XML_SCHEMA_URI,
                category: "xml-document",
                output_syntax: ConversionOutputSyntax::Xml,
                expected_syntax: TransformTemplateTargetSyntaxKind::Xml,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "HTML",
                content_type: HTML_CONTENT_TYPE,
                schema: HTML_SCHEMA_URI,
                category: "html-fragment",
                output_syntax: ConversionOutputSyntax::Html,
                expected_syntax: TransformTemplateTargetSyntaxKind::Html,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "JSON family",
                content_type: JSON_CONTENT_TYPE,
                schema: JSON_VALUE_SCHEMA_URI,
                category: "json-document",
                output_syntax: ConversionOutputSyntax::Json,
                expected_syntax: TransformTemplateTargetSyntaxKind::Json,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "YAML",
                content_type: YAML_CONTENT_TYPE,
                schema: YAML_SCHEMA_URI,
                category: "yaml-document",
                output_syntax: ConversionOutputSyntax::Yaml,
                expected_syntax: TransformTemplateTargetSyntaxKind::Yaml,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "CSV",
                content_type: CSV_CONTENT_TYPE,
                schema: CSV_SCHEMA_URI,
                category: "csv-record",
                output_syntax: ConversionOutputSyntax::Csv,
                expected_syntax: TransformTemplateTargetSyntaxKind::Csv,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "Markdown",
                content_type: MARKDOWN_CONTENT_TYPE,
                schema: MARKDOWN_SCHEMA_URI,
                category: "markdown-document",
                output_syntax: ConversionOutputSyntax::Markdown,
                expected_syntax: TransformTemplateTargetSyntaxKind::Markdown,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "CSS",
                content_type: CSS_CONTENT_TYPE,
                schema: CSS_SCHEMA_URI,
                category: "css-stylesheet",
                output_syntax: ConversionOutputSyntax::Css,
                expected_syntax: TransformTemplateTargetSyntaxKind::Css,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "Terminal color text",
                content_type: "text/plain",
                schema: "https://cem.dev/ns/data/text/1",
                category: "terminal-color",
                output_syntax: ConversionOutputSyntax::Text,
                expected_syntax: TransformTemplateTargetSyntaxKind::Text,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "HTML color output",
                content_type: HTML_CONTENT_TYPE,
                schema: HTML_SCHEMA_URI,
                category: "html-color-fragment",
                output_syntax: ConversionOutputSyntax::Html,
                expected_syntax: TransformTemplateTargetSyntaxKind::Html,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "CEM-QL",
                content_type: CEM_QL_CONTENT_TYPE,
                schema: CEM_QL_SCHEMA_URI,
                category: "cem-ql-module",
                output_syntax: ConversionOutputSyntax::Text,
                expected_syntax: TransformTemplateTargetSyntaxKind::Text,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "RELAX NG compact",
                content_type: RELAX_NG_COMPACT_CONTENT_TYPE,
                schema: RELAX_NG_SCHEMA_URI,
                category: "rnc-document",
                output_syntax: ConversionOutputSyntax::Text,
                expected_syntax: TransformTemplateTargetSyntaxKind::Text,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "AI context projections",
                content_type: AI_CONTEXT_JSON_CONTENT_TYPE,
                schema: AI_CONTEXT_SCHEMA_URI,
                category: "ai-context-pack",
                output_syntax: ConversionOutputSyntax::Json,
                expected_syntax: TransformTemplateTargetSyntaxKind::Json,
                expected_produces: TransformTemplateOutputProducedKind::Text,
            },
            FamilyCase {
                family: "CEM binary projections",
                content_type: CEM_AST_PROJECTION_CONTENT_TYPE,
                schema: CEM_AST_PROJECTION_SCHEMA_URI,
                category: "cem-bin-document",
                output_syntax: ConversionOutputSyntax::Binary,
                expected_syntax: TransformTemplateTargetSyntaxKind::Binary,
                expected_produces: TransformTemplateOutputProducedKind::Bytes,
            },
        ];

        for (index, case) in cases.iter().enumerate() {
            let descriptor = cemt_edge_with_output_contract(
                &format!("proposal-family-{index}"),
                endpoint(case.content_type, case.schema),
                ConversionOutputContractDescriptor {
                    output_syntax: Some(case.output_syntax),
                    encoding_category: Some(case.category.to_owned()),
                    ..ConversionOutputContractDescriptor::default()
                },
            );

            let (contract, diagnostics) = conversion_output_safety_contract(&descriptor);

            assert!(diagnostics.is_empty(), "{}: {diagnostics:?}", case.family);
            assert_eq!(
                conversion_encoding_category_syntax(case.category),
                Some(case.expected_syntax),
                "{}",
                case.family
            );
            let contract = contract.unwrap_or_else(|| panic!("{}: missing contract", case.family));
            assert_eq!(contract.target.content_type, case.content_type);
            assert_eq!(contract.target.schema, case.schema);
            assert_eq!(contract.target.category, case.category);
            assert_eq!(contract.syntax_rules.syntax, case.expected_syntax);
            assert_eq!(contract.produces, case.expected_produces);
            assert_eq!(
                contract.insertion_context.category.as_deref(),
                Some(case.category)
            );
        }
    }

    #[test]
    fn output_safety_contracts_report_missing_and_unsupported_metadata() {
        let mut registry = ConversionRegistry::new();
        registry
            .register(cemt_edge_with_output_contract(
                "missing-output-metadata",
                endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                ConversionOutputContractDescriptor::default(),
            ))
            .unwrap();
        registry
            .register(cemt_edge_with_output_contract(
                "unknown-category",
                endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                ConversionOutputContractDescriptor {
                    output_syntax: Some(ConversionOutputSyntax::Html),
                    encoding_category: Some("unknown-context".to_owned()),
                    ..ConversionOutputContractDescriptor::default()
                },
            ))
            .unwrap();

        let (contracts, diagnostics) = registry.cemt_output_safety_contracts();

        assert!(contracts.is_empty());
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CONVERSION_OUTPUT_SYNTAX_MISSING_CODE));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CONVERSION_OUTPUT_UNSUPPORTED_CATEGORY_CODE));
    }

    #[test]
    fn output_safety_contracts_report_category_target_and_color_mismatches() {
        let mut registry = ConversionRegistry::new();
        registry
            .register(cemt_edge_with_output_contract(
                "html-syntax-with-xml-category",
                endpoint(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                ConversionOutputContractDescriptor {
                    output_syntax: Some(ConversionOutputSyntax::Html),
                    encoding_category: Some("xml-text".to_owned()),
                    ..ConversionOutputContractDescriptor::default()
                },
            ))
            .unwrap();
        registry
            .register(cemt_edge_with_output_contract(
                "xml-with-html-color",
                endpoint(XML_CONTENT_TYPE, XML_SCHEMA_URI),
                ConversionOutputContractDescriptor {
                    output_syntax: Some(ConversionOutputSyntax::Xml),
                    encoding_category: Some("xml-document".to_owned()),
                    color_profile: Some("classes".to_owned()),
                    ..ConversionOutputContractDescriptor::default()
                },
            ))
            .unwrap();

        let (contracts, diagnostics) = registry.cemt_output_safety_contracts();

        assert!(contracts.is_empty());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE
                && diagnostic.message.contains("xml-text")
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == CONVERSION_OUTPUT_CONTEXT_MISMATCH_CODE
                && diagnostic.message.contains("HTML color profile")
        }));
    }

    #[test]
    fn output_safety_contracts_accept_baseline_color_profile_aliases() {
        struct ColorAliasCase {
            id: &'static str,
            content_type: &'static str,
            schema: &'static str,
            output_syntax: ConversionOutputSyntax,
            category: &'static str,
            color_profile: &'static str,
            expected_output: TransformTemplateColorOutputKind,
        }

        let cases = [
            ColorAliasCase {
                id: "terminal-color-alias",
                content_type: "text/plain",
                schema: "https://cem.dev/ns/data/text/1",
                output_syntax: ConversionOutputSyntax::Text,
                category: "terminal-color",
                color_profile: "terminal",
                expected_output: TransformTemplateColorOutputKind::Terminal,
            },
            ColorAliasCase {
                id: "html-color-alias",
                content_type: HTML_CONTENT_TYPE,
                schema: HTML_SCHEMA_URI,
                output_syntax: ConversionOutputSyntax::Html,
                category: "html-document",
                color_profile: "html",
                expected_output: TransformTemplateColorOutputKind::Html,
            },
            ColorAliasCase {
                id: "markdown-color-alias",
                content_type: MARKDOWN_CONTENT_TYPE,
                schema: MARKDOWN_SCHEMA_URI,
                output_syntax: ConversionOutputSyntax::Markdown,
                category: "markdown-document",
                color_profile: "md",
                expected_output: TransformTemplateColorOutputKind::None,
            },
        ];

        for case in cases {
            let descriptor = cemt_edge_with_output_contract(
                case.id,
                endpoint(case.content_type, case.schema),
                ConversionOutputContractDescriptor {
                    output_syntax: Some(case.output_syntax),
                    encoding_category: Some(case.category.to_owned()),
                    color_profile: Some(case.color_profile.to_owned()),
                    ..ConversionOutputContractDescriptor::default()
                },
            );

            let (contract, diagnostics) = conversion_output_safety_contract(&descriptor);

            assert!(diagnostics.is_empty(), "{}: {diagnostics:?}", case.id);
            assert_eq!(
                contract
                    .and_then(|contract| contract.color_output_profile)
                    .map(|profile| profile.output),
                Some(case.expected_output),
                "{}",
                case.id
            );
        }
    }

    #[test]
    fn output_safety_contract_context_drives_encoded_artifact_guard() {
        let registry = ConversionRegistry::with_builtin_converters();
        let (contracts, diagnostics) = registry.cemt_output_safety_contracts();
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let html = contracts
            .iter()
            .find(|contract| contract.descriptor.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML output safety contract");
        assert_eq!(
            html.pipeline.writer_insertion_context,
            html.insertion_context
        );

        let mut identity =
            TransformTemplateEncodedArtifactIdentity::new(html.produces, html.target.clone());
        identity.formatter_profile = html.insertion_context.formatter_profile.clone();
        identity.color_profile = html.insertion_context.color_profile.clone();
        identity.mode = TransformTemplateEncodedArtifactMode::Document;
        identity.canonical = true;
        let artifact =
            TransformTemplateEncodedArtifact::new(identity, Value::String("<main></main>".into()));

        artifact
            .validate_insertion(&html.insertion_context)
            .expect("matching conversion safety context accepts artifact");
        let double_encoding = artifact
            .validate_as_encode_input()
            .expect_err("encoded artifacts cannot be encoded again");
        assert_eq!(
            double_encoding.code(),
            TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_DOUBLE_ENCODING_CODE
        );

        let mut cemt_identity = TransformTemplateEncodedArtifactIdentity::new(
            html.pipeline.cemt_produces,
            html.pipeline.cemt_target.clone(),
        );
        cemt_identity.formatter_profile = html
            .pipeline
            .cemt_insertion_context
            .formatter_profile
            .clone();
        cemt_identity.color_profile = html.pipeline.cemt_insertion_context.color_profile.clone();
        cemt_identity.mode = TransformTemplateEncodedArtifactMode::Document;
        cemt_identity.canonical = true;
        let cemt_tree = TransformTemplateEncodedArtifact::new(
            cemt_identity,
            serde_json::json!({
                "kind": "cem-tree",
                "contentType": CEM_ML_CONTENT_TYPE,
                "schema": CEM_ML_SCHEMA_URI,
                "category": "cem-tree",
                "mode": "document",
                "canonical": true,
                "formatterProfile": "compact",
                "formatNodes": [{
                    "kind": "format-marker",
                    "name": "cem.format-tree",
                    "formatterRole": "formatter.boundary",
                    "formatterProfile": "compact"
                }, {
                    "kind": "format-decision",
                    "name": "line-ending",
                    "formatterRole": "formatter.line-ending",
                    "value": "lf",
                    "formatterProfile": "compact"
                }],
                "colored": true,
                "colorProfile": "classes",
                "colorNodes": [{
                    "kind": "color-marker",
                    "name": "cem.color-tree",
                    "colorizerRole": "colorizer.boundary",
                    "colorProfile": "classes"
                }, {
                    "kind": "color-decision",
                    "name": "profile",
                    "colorizerRole": "colorizer.profile",
                    "value": "classes",
                    "colorProfile": "classes"
                }],
                "nodes": [{
                    "kind": "element",
                    "name": "main",
                    "colorRole": "syntax.name",
                    "writerAttributeNodes": [{
                        "kind": "writer-attribute",
                        "name": "class",
                        "value": "cem-color cem-color-syntax-name",
                        "colorizerOwned": true,
                        "colorizerRole": "colorizer.writer-attribute",
                        "colorProfile": "classes"
                    }],
                    "children": [{"kind": "text", "value": "Ready"}]
                }]
            }),
        );
        cemt_tree
            .validate_insertion(&html.pipeline.cemt_insertion_context)
            .expect("CEMT pipeline accepts formatted and colored CEM tree");

        let mut wrong_category = html.insertion_context.clone();
        wrong_category.category = Some("html-text".to_owned());
        let mismatch = artifact
            .validate_insertion(&wrong_category)
            .expect_err("category mismatch is rejected");
        assert_eq!(
            mismatch.code(),
            TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_CONTEXT_MISMATCH_CODE
        );
    }

    #[test]
    fn conversion_output_pipeline_exposes_colored_cem_tree_before_writer() {
        let registry = ConversionRegistry::with_builtin_converters();
        let (contracts, diagnostics) = registry.cemt_output_safety_contracts();
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let html = contracts
            .iter()
            .find(|contract| contract.descriptor.id == "cem-dom-projection-to-html-cemt")
            .expect("HTML output safety contract");

        let execution = execute_conversion_output_pipeline(
            &html.pipeline,
            serde_json::json!({
                "kind": "element",
                "name": "main",
                "children": [{"kind": "text", "value": "Ready"}]
            }),
            None,
            Vec::new(),
            "cem-dom-projection-to-html-cemt",
            Some("basic-dom"),
            Some("schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert!(execution.format_elapsed_ns.is_some());
        assert!(execution.color_elapsed_ns.is_some());
        assert!(execution.writer_elapsed_ns.is_some());
        assert_eq!(
            execution.format_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_FORMAT_CEMT_ADAPTER_ID.to_owned(),
                function_name: "cem.format-tree".to_owned(),
                body_function_name: Some("cem.format-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        assert_eq!(
            execution.color_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_COLOR_CEMT_ADAPTER_ID.to_owned(),
                function_name: "cem.color-tree".to_owned(),
                body_function_name: Some("cem.color-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted CEM tree stage is retained");
        assert_eq!(
            formatted.identity.produces,
            TransformTemplateOutputProducedKind::CemTree
        );
        assert_eq!(formatted.value["kind"], "cem-tree");
        assert_eq!(formatted.value["formatterProfile"], "compact");
        assert!(formatted.value.get("formatNodes").is_some());
        assert!(formatted.value.get("colored").is_none());
        assert!(formatted.value.get("colorNodes").is_none());
        assert!(formatted.value["nodes"][0]
            .get("writerAttributeNodes")
            .is_none());

        let colored = execution
            .colored_cem_tree
            .as_ref()
            .expect("colored CEM tree stage is retained");
        assert_eq!(
            colored.identity.produces,
            TransformTemplateOutputProducedKind::CemTree
        );
        assert_eq!(colored.value["kind"], "cem-tree");
        assert_eq!(colored.value["colored"], true);
        assert_eq!(colored.value["colorProfile"], "classes");
        assert_eq!(colored.value["colorNodes"][0]["name"], "cem.color-tree");
        assert_eq!(
            colored.value["nodes"][0]["writerAttributeNodes"][0]["kind"],
            "writer-attribute"
        );
        assert_eq!(
            colored.value["nodes"][0]["children"][0]["colorWrapperNodes"][0]["kind"],
            "color-wrapper"
        );

        let output = execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("writer output text");
        assert!(output.contains("<main"));
        assert!(output.contains("cem-color-syntax-name"));
        assert!(output.contains("<span class=\"cem-color cem-color-syntax-string\""));
    }

    #[test]
    fn conversion_output_pipeline_applies_literal_baseline_formatter_profiles() {
        let mut pretty_pipeline = direct_cem_output_pipeline();
        pretty_pipeline.cemt_options.formatter_profile = Some("pretty".to_owned());
        pretty_pipeline.cemt_insertion_context.formatter_profile = Some("pretty".to_owned());
        pretty_pipeline.writer_insertion_context.formatter_profile = Some("pretty".to_owned());
        let pretty_execution = execute_conversion_output_pipeline(
            &pretty_pipeline,
            serde_json::json!({
                "kind": "element",
                "name": "card",
                "children": [
                    {"kind": "element", "name": "title", "children": [{"kind": "text", "value": "Ready"}]},
                    {"kind": "element", "name": "body", "children": [{"kind": "text", "value": "Now"}]}
                ]
            }),
            None,
            Vec::new(),
            "test-pretty-cem-output",
            Some("pretty-dom"),
            Some("converter.cemt"),
        );

        assert!(
            pretty_execution.diagnostics.is_empty(),
            "{:?}",
            pretty_execution.diagnostics
        );
        assert_eq!(
            pretty_execution.format_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_FORMAT_CEMT_ADAPTER_ID.to_owned(),
                function_name: "cem.format-tree".to_owned(),
                body_function_name: Some("cem.format-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        assert_eq!(pretty_execution.color_execution, None);
        assert_eq!(pretty_execution.color_elapsed_ns, None);
        let pretty_formatted = pretty_execution
            .formatted_cem_tree
            .as_ref()
            .expect("pretty formatted tree");
        assert_eq!(pretty_formatted.value["formatterProfile"], "pretty");
        assert_eq!(
            pretty_formatted.value["nodes"][0]["formatLayout"]["value"],
            "block"
        );
        assert_eq!(
            pretty_formatted.value["nodes"][0]["formatContentBoundary"][1]["formatterRole"],
            "formatter.content-boundary"
        );
        assert_eq!(
            pretty_formatted.value["nodes"][0]["formatBeforeClose"]["formatterRole"],
            "formatter.close-indent"
        );
        assert_eq!(
            pretty_formatted.value["nodes"][0]["children"][0]["formatterRole"],
            "formatter.indent"
        );
        assert_eq!(
            pretty_formatted.value["nodes"][0]["children"][1]["name"],
            "title"
        );
        assert_eq!(
            pretty_formatted.value["nodes"][0]["children"][2]["formatterRole"],
            "formatter.line-ending"
        );
        let pretty_output = pretty_execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("pretty writer output");
        assert!(pretty_output.contains("\n    {title"));

        let mut tabular_pipeline = direct_cem_output_pipeline();
        tabular_pipeline.cemt_options.formatter_profile = Some("tabular".to_owned());
        tabular_pipeline.cemt_insertion_context.formatter_profile = Some("tabular".to_owned());
        tabular_pipeline.writer_insertion_context.formatter_profile = Some("tabular".to_owned());
        let tabular_execution = execute_conversion_output_pipeline(
            &tabular_pipeline,
            serde_json::json!({
                "kind": "element",
                "name": "card",
                "attributes": [
                    {"kind": "attribute", "name": "tone", "value": "info"},
                    {"kind": "attribute", "name": "size", "value": "lg"}
                ],
                "children": [{"kind": "text", "value": "Ready"}]
            }),
            None,
            Vec::new(),
            "test-tabular-cem-output",
            Some("tabular-dom"),
            Some("converter.cemt"),
        );

        assert!(
            tabular_execution.diagnostics.is_empty(),
            "{:?}",
            tabular_execution.diagnostics
        );
        assert_eq!(tabular_execution.color_execution, None);
        assert_eq!(tabular_execution.color_elapsed_ns, None);
        let tabular_formatted = tabular_execution
            .formatted_cem_tree
            .as_ref()
            .expect("tabular formatted tree");
        assert_eq!(tabular_formatted.value["formatterProfile"], "tabular");
        assert_eq!(
            tabular_formatted.value["nodes"][0]["formatBeforeAttributes"]["value"],
            " "
        );
        assert_eq!(
            tabular_formatted.value["nodes"][0]["formatBetweenAttributes"]["formatterRole"],
            "formatter.attribute-spacing"
        );
        assert_eq!(
            tabular_formatted.value["nodes"][0]["formatBetweenAttributes"]["value"],
            " "
        );
        let tabular_output = tabular_execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("tabular writer output");
        assert!(tabular_output.contains("{card @tone=info @size=lg |"));

        tabular_pipeline.cemt_options.wrap_column = Some("24".to_owned());
        let wrapped_tabular_execution = execute_conversion_output_pipeline(
            &tabular_pipeline,
            serde_json::json!({
                "kind": "element",
                "name": "card",
                "attributes": [
                    {"kind": "attribute", "name": "tone", "value": "info"},
                    {"kind": "attribute", "name": "size", "value": "lg"}
                ],
                "children": [{"kind": "text", "value": "Ready"}]
            }),
            None,
            Vec::new(),
            "test-tabular-cem-output-wrapped",
            Some("tabular-dom"),
            Some("converter.cemt"),
        );

        assert!(
            wrapped_tabular_execution.diagnostics.is_empty(),
            "{:?}",
            wrapped_tabular_execution.diagnostics
        );
        let wrapped_tabular_formatted = wrapped_tabular_execution
            .formatted_cem_tree
            .as_ref()
            .expect("wrapped tabular formatted tree");
        assert_eq!(
            wrapped_tabular_formatted.value["nodes"][0]["formatBeforeAttributes"]["value"],
            " "
        );
        assert_eq!(
            wrapped_tabular_formatted.value["nodes"][0]["formatBetweenAttributes"]["formatterRole"],
            "formatter.attribute-spacing"
        );
        assert_eq!(
            wrapped_tabular_formatted.value["nodes"][0]["attributes"][1]["formatBefore"][0]
                ["formatterRole"],
            "formatter.attribute-spacing"
        );
        assert_eq!(
            wrapped_tabular_formatted.value["nodes"][0]["attributes"][1]["formatBefore"][0]
                ["value"],
            "\n"
        );
        assert_eq!(
            wrapped_tabular_formatted.value["nodes"][0]["attributes"][1]["formatBefore"][1]
                ["formatterRole"],
            "formatter.attribute-indent"
        );
        let wrapped_tabular_output = wrapped_tabular_execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("wrapped tabular writer output");
        assert!(wrapped_tabular_output.contains("{card @tone=info\n    @size=lg"));
    }

    #[test]
    fn conversion_output_pipeline_applies_literal_baseline_colorizer_profiles() {
        for profile in ["terminal", "md"] {
            let mut pipeline = if profile == "md" {
                direct_markup_output_pipeline(
                    ConversionOutputSyntax::Markdown,
                    MARKDOWN_CONTENT_TYPE,
                    MARKDOWN_SCHEMA_URI,
                    "markdown-document",
                    Some("md"),
                )
            } else {
                direct_cem_output_pipeline()
            };
            pipeline.cemt_options.color_profile = Some(profile.to_owned());
            pipeline.cemt_insertion_context.color_profile = Some(profile.to_owned());
            pipeline.writer_insertion_context.color_profile = Some(profile.to_owned());

            let execution = execute_conversion_output_pipeline(
                &pipeline,
                serde_json::json!({
                    "kind": "element",
                    "name": "card",
                    "children": [{"kind": "text", "value": "Ready"}]
                }),
                None,
                Vec::new(),
                format!("test-{profile}-cem-output").as_str(),
                Some("literal-colorizer"),
                Some("converter.cemt"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            let colored = execution
                .colored_cem_tree
                .as_ref()
                .expect("colored CEM tree");
            assert_eq!(colored.value["colorProfile"], profile);
            assert_eq!(
                colored.value["colorOutput"],
                if profile == "terminal" {
                    "terminal"
                } else {
                    "md"
                }
            );
            assert!(colored.value["nodes"][0]
                .get("writerAttributeNodes")
                .is_none());
            let output = execution
                .output
                .as_ref()
                .and_then(Value::as_str)
                .expect("writer output");
            if profile == "terminal" {
                assert!(output.contains("\u{1b}[38;5;81mcard\u{1b}[0m"));
                assert!(output.contains("\u{1b}[38;5;76mReady\u{1b}[0m"));
            } else {
                assert!(output.contains(
                    r#"<span class="cem-color cem-color-syntax-name" data-role="syntax.name">card</span>"#
                ));
                assert!(output.contains(
                    r#"<span class="cem-color cem-color-syntax-string" data-role="syntax.string">Ready</span>"#
                ));
            }
        }

        let mut html_pipeline = direct_html_output_pipeline();
        html_pipeline.cemt_options.color_profile = Some("html".to_owned());
        html_pipeline.cemt_insertion_context.color_profile = Some("html".to_owned());
        html_pipeline.writer_insertion_context.color_profile = Some("html".to_owned());
        let html_execution = execute_conversion_output_pipeline(
            &html_pipeline,
            serde_json::json!({
                "kind": "element",
                "name": "card",
                "children": [{"kind": "text", "value": "Ready"}]
            }),
            None,
            Vec::new(),
            "test-html-cem-output",
            Some("literal-colorizer"),
            Some("converter.cemt"),
        );

        assert!(
            html_execution.diagnostics.is_empty(),
            "{:?}",
            html_execution.diagnostics
        );
        let html_colored = html_execution
            .colored_cem_tree
            .as_ref()
            .expect("HTML colored CEM tree");
        assert_eq!(html_colored.value["colorProfile"], "html");
        assert_eq!(html_colored.value["colorOutput"], "html");
        assert_eq!(
            html_colored.value["nodes"][0]["style"]["htmlMode"],
            "classes"
        );
        let html_output = html_execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("HTML writer output");
        assert!(html_output.contains("cem-color-syntax-name"));
        assert!(html_output.contains("<span class=\"cem-color cem-color-syntax-string\""));
    }

    #[test]
    fn conversion_output_pipeline_reads_cemt_artifacts_through_environment_reader() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let mut conversion_registry = ConversionRegistry::new();
        let formatter_path = "cem+test://packages/cem-ml/v1/formatters/cem-format-tree.cemt";
        let formatter_helper_path =
            "cem+test://packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt";
        let colorizer_path = "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree.cemt";
        let colorizer_helper_path =
            "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt";
        conversion_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "formatter".to_owned(),
            path: formatter_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.format-tree".to_owned()),
            function_profile: None,
            formatter_profile: Some("compact".to_owned()),
            color_profile: None,
            generated: false,
        });
        conversion_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND.to_owned(),
            path: formatter_helper_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.format-tree.apply-stage".to_owned()),
            function_profile: Some("cem.format-tree".to_owned()),
            formatter_profile: Some("compact".to_owned()),
            color_profile: None,
            generated: false,
        });
        conversion_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "colorizer".to_owned(),
            path: colorizer_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.color-tree".to_owned()),
            function_profile: Some("css-custom-properties".to_owned()),
            formatter_profile: None,
            color_profile: Some("classes".to_owned()),
            generated: false,
        });
        conversion_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND.to_owned(),
            path: colorizer_helper_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.color-tree.apply-stage".to_owned()),
            function_profile: Some("css-custom-properties".to_owned()),
            formatter_profile: None,
            color_profile: None,
            generated: false,
        });
        let formatter_source = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt",
        )
        .expect("embedded formatter source");
        let formatter_helper_source = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt",
        )
        .expect("embedded formatter helper source");
        let colorizer_source = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt",
        )
        .expect("embedded colorizer source");
        let colorizer_helper_source = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt",
        )
        .expect("embedded colorizer helper source");
        let reads = std::cell::RefCell::new(Vec::new());
        let package_artifact_reader =
            |artifact: &ConversionPackageArtifactDescriptor| -> Result<ConversionPackageArtifactRead, String> {
                reads.borrow_mut().push(artifact.path.clone());
                let source = match artifact.path.as_str() {
                    path if path == formatter_path => formatter_source.source,
                    path if path == formatter_helper_path => formatter_helper_source.source,
                    path if path == colorizer_path => colorizer_source.source,
                    path if path == colorizer_helper_path => colorizer_helper_source.source,
                    other => return Err(format!("unexpected artifact path `{other}`")),
                };
                Ok(ConversionPackageArtifactRead {
                    uri: artifact.path.clone(),
                    bytes: source.as_bytes().to_vec(),
                    content_type: artifact.content_type.clone(),
                })
            };
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: Some(&package_artifact_reader),
            artifact_cache: None,
        };

        let execution = execute_conversion_output_pipeline_with_environment(
            &environment,
            &direct_html_output_pipeline(),
            serde_json::json!({
                "kind": "element",
                "name": "main",
                "children": [{"kind": "text", "value": "Ready"}]
            }),
            None,
            Vec::new(),
            "test-dom-to-html-cemt",
            Some("output"),
            Some("test.cem"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            reads.into_inner(),
            vec![
                formatter_helper_path.to_owned(),
                colorizer_helper_path.to_owned(),
                formatter_path.to_owned(),
                colorizer_path.to_owned(),
            ]
        );
        assert_eq!(
            execution.format_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_FORMAT_CEMT_ADAPTER_ID.to_owned(),
                function_name: "cem.format-tree".to_owned(),
                body_function_name: Some("cem.format-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        assert_eq!(
            execution.color_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_COLOR_CEMT_ADAPTER_ID.to_owned(),
                function_name: "cem.color-tree".to_owned(),
                body_function_name: Some("cem.color-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        let output = execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("writer output text");
        assert!(output.contains("<main"));
        assert!(output.contains("cem-color-syntax-name"));
    }

    #[test]
    fn conversion_output_pipeline_reports_missing_cemt_helper_artifact() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let mut conversion_registry = ConversionRegistry::new();
        let formatter_path = "cem+test://packages/cem-ml/v1/formatters/cem-format-tree.cemt";
        conversion_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "formatter".to_owned(),
            path: formatter_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.format-tree".to_owned()),
            function_profile: None,
            formatter_profile: Some("compact".to_owned()),
            color_profile: None,
            generated: false,
        });
        let formatter_source = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt",
        )
        .expect("embedded formatter source");
        let package_artifact_reader =
            |artifact: &ConversionPackageArtifactDescriptor| -> Result<ConversionPackageArtifactRead, String> {
                let source = match artifact.path.as_str() {
                    path if path == formatter_path => formatter_source.source,
                    other => return Err(format!("unexpected artifact path `{other}`")),
                };
                Ok(ConversionPackageArtifactRead {
                    uri: artifact.path.clone(),
                    bytes: source.as_bytes().to_vec(),
                    content_type: artifact.content_type.clone(),
                })
            };
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: Some(&package_artifact_reader),
            artifact_cache: None,
        };

        let execution = execute_conversion_output_pipeline_with_environment(
            &environment,
            &direct_html_output_pipeline(),
            serde_json::json!({
                "kind": "element",
                "name": "main",
                "children": [{"kind": "text", "value": "Ready"}]
            }),
            None,
            Vec::new(),
            "test-dom-to-html-cemt",
            Some("output"),
            Some("test.cem"),
        );

        let message = execution
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.as_str())
            .expect("missing helper diagnostic");
        assert!(message.contains("CEMT formatter `cem.format-tree` failed"));
        assert!(message.contains("requires helper function `cem.format-tree.apply-stage`"));
        assert!(message.contains("no matching `formatter-helper` artifact was loaded"));
        assert!(!message.contains("body expression could not be resolved"));
    }

    #[test]
    fn conversion_output_pipeline_executes_showcase_cemt_artifacts_through_registry() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let mut conversion_registry = ConversionRegistry::new();
        let formatter_path =
            "cem+test://packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt";
        let colorizer_path =
            "cem+test://packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt";
        conversion_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "formatter".to_owned(),
            path: formatter_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("acme.showcase.format-tree".to_owned()),
            function_profile: None,
            formatter_profile: Some("acme.showcase.format-tree".to_owned()),
            color_profile: None,
            generated: false,
        });
        conversion_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "colorizer".to_owned(),
            path: colorizer_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("acme.showcase.color-tree".to_owned()),
            function_profile: Some("classes".to_owned()),
            formatter_profile: None,
            color_profile: Some("classes".to_owned()),
            generated: false,
        });

        let showcase_source = include_str!(
            "../schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.cemt"
        );
        let reads = std::cell::RefCell::new(Vec::new());
        let package_artifact_reader =
            |artifact: &ConversionPackageArtifactDescriptor| -> Result<ConversionPackageArtifactRead, String> {
                reads.borrow_mut().push(artifact.path.clone());
                match artifact.path.as_str() {
                    path if path == formatter_path || path == colorizer_path => {
                        Ok(ConversionPackageArtifactRead {
                            uri: artifact.path.clone(),
                            bytes: showcase_source.as_bytes().to_vec(),
                            content_type: artifact.content_type.clone(),
                        })
                    }
                    other => Err(format!("unexpected artifact path `{other}`")),
                }
            };
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: Some(&package_artifact_reader),
            artifact_cache: None,
        };

        let mut pipeline = direct_html_output_pipeline();
        pipeline.cemt_options.formatter_profile = Some("acme.showcase.format-tree".to_owned());
        pipeline.cemt_insertion_context.formatter_profile =
            Some("acme.showcase.format-tree".to_owned());
        pipeline.writer_insertion_context.formatter_profile =
            Some("acme.showcase.format-tree".to_owned());

        let execution = execute_conversion_output_pipeline_with_environment(
            &environment,
            &pipeline,
            serde_json::json!({
                "kind": "element",
                "name": "article",
                "sourceMap": null,
                "attributes": [],
                "children": [
                    {"kind": "text", "value": "Ready ", "sourceMap": null},
                    {
                        "kind": "element",
                        "name": "strong",
                        "sourceMap": null,
                        "attributes": [],
                        "children": [{"kind": "text", "value": "now", "sourceMap": null}]
                    },
                    {"kind": "text", "value": ".", "sourceMap": null}
                ]
            }),
            None,
            Vec::new(),
            "test-storybook-cemt-pipeline",
            Some("output"),
            Some("storybook.cem"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            reads.into_inner(),
            vec![formatter_path.to_owned(), colorizer_path.to_owned()]
        );
        assert_eq!(
            execution.format_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_FORMAT_CEMT_ADAPTER_ID.to_owned(),
                function_name: "acme.showcase.format-tree".to_owned(),
                body_function_name: Some("acme.showcase.format-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        assert_eq!(
            execution.color_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_COLOR_CEMT_ADAPTER_ID.to_owned(),
                function_name: "acme.showcase.color-tree".to_owned(),
                body_function_name: Some("acme.showcase.color-tree".to_owned()),
                fallback_function_name: None,
            })
        );

        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted CEM tree stage is retained");
        assert_eq!(
            formatted.value["formatterProfile"],
            "acme.showcase.format-tree"
        );
        assert!(formatted.value.get("colored").is_none());
        assert!(formatted.value.get("colorNodes").is_none());
        assert!(formatted.value["formatNodes"]
            .as_array()
            .expect("format nodes")
            .iter()
            .any(|node| node.get("value").and_then(Value::as_str)
                == Some("formatted tree before writer")));

        let colored = execution
            .colored_cem_tree
            .as_ref()
            .expect("colored CEM tree stage is retained");
        assert_eq!(colored.value["colored"], true);
        assert_eq!(colored.value["colorProfile"], "classes");
        assert!(colored.value["colorNodes"]
            .as_array()
            .expect("color nodes")
            .iter()
            .any(|node| node.get("value").and_then(Value::as_str)
                == Some("colored tree before writer")));
        assert_eq!(
            colored.value["nodes"][0]["writerAttributeNodes"][0]["kind"],
            "writer-attribute"
        );
        assert_eq!(
            colored.value["nodes"][0]["children"][1]["colorRole"],
            "syntax.keyword"
        );

        let output = execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("writer output text");
        assert!(output.contains("<article"));
        assert!(output.contains("cem-color-syntax-name"));
        assert!(output.contains("<strong class=\"cem-color cem-color-syntax-keyword\""));
        assert!(output.contains("Ready "));
        assert!(output.contains(">now<"));
    }

    #[test]
    fn conversion_output_pipeline_executes_manifest_declared_showcase_cemt_artifacts() {
        let mut pipeline = direct_html_output_pipeline();
        pipeline.cemt_options.formatter = Some("acme.showcase.format-tree".to_owned());
        pipeline.cemt_options.formatter_profile = Some("acme.showcase.format-tree".to_owned());
        pipeline.cemt_options.colorizer = Some("acme.showcase.color-tree".to_owned());
        pipeline.cemt_options.color_profile = Some("classes".to_owned());
        pipeline.cemt_insertion_context.formatter_profile =
            Some("acme.showcase.format-tree".to_owned());
        pipeline.writer_insertion_context.formatter_profile =
            Some("acme.showcase.format-tree".to_owned());

        let execution = execute_conversion_output_pipeline(
            &pipeline,
            serde_json::json!({
                "kind": "element",
                "name": "article",
                "sourceMap": null,
                "attributes": [],
                "children": [
                    {"kind": "text", "value": "Ready ", "sourceMap": null},
                    {
                        "kind": "element",
                        "name": "strong",
                        "sourceMap": null,
                        "attributes": [],
                        "children": [{"kind": "text", "value": "now", "sourceMap": null}]
                    },
                    {"kind": "text", "value": ".", "sourceMap": null}
                ]
            }),
            None,
            Vec::new(),
            "test-manifest-cemt-pipeline",
            Some("output"),
            Some("manifest-storybook.cem"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.format_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_FORMAT_CEMT_ADAPTER_ID.to_owned(),
                function_name: "acme.showcase.format-tree".to_owned(),
                body_function_name: Some("acme.showcase.format-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        assert_eq!(
            execution.color_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_COLOR_CEMT_ADAPTER_ID.to_owned(),
                function_name: "acme.showcase.color-tree".to_owned(),
                body_function_name: Some("acme.showcase.color-tree".to_owned()),
                fallback_function_name: None,
            })
        );

        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted CEM tree stage is retained");
        assert_eq!(
            formatted.value["formatterProfile"],
            "acme.showcase.format-tree"
        );
        assert!(formatted.value["formatNodes"]
            .as_array()
            .expect("format nodes")
            .iter()
            .any(|node| node.get("value").and_then(Value::as_str)
                == Some("formatted tree before writer")));

        let colored = execution
            .colored_cem_tree
            .as_ref()
            .expect("colored CEM tree stage is retained");
        assert_eq!(colored.value["colored"], true);
        assert_eq!(colored.value["colorProfile"], "classes");
        assert!(colored.value["colorNodes"]
            .as_array()
            .expect("color nodes")
            .iter()
            .any(|node| node.get("value").and_then(Value::as_str)
                == Some("colored tree before writer")));

        let output = execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("writer output text");
        assert!(output.contains("<article"));
        assert!(output.contains("cem-color-syntax-name"));
        assert!(output.contains("<strong class=\"cem-color cem-color-syntax-keyword\""));
        assert!(output.contains("Ready "));
        assert!(output.contains(">now<"));
    }

    #[test]
    fn conversion_output_pipeline_colors_converter_formatted_cem_tree() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let mut pipeline = direct_html_output_pipeline();
        pipeline.cemt_options.formatter = Some("acme.showcase.format-tree".to_owned());
        pipeline.cemt_options.formatter_profile = Some("acme.showcase.format-tree".to_owned());
        pipeline.cemt_options.colorizer = Some("acme.showcase.color-tree".to_owned());
        pipeline.cemt_options.color_profile = Some("classes".to_owned());
        pipeline.cemt_insertion_context.formatter_profile =
            Some("acme.showcase.format-tree".to_owned());
        pipeline.writer_insertion_context.formatter_profile =
            Some("acme.showcase.format-tree".to_owned());

        let execution = execute_conversion_output_pipeline_from_formatted_cem_tree_with_environment(
            &environment,
            &pipeline,
            serde_json::json!([
                {
                    "kind": "element",
                    "name": "article",
                    "children": [
                        {"kind": "text", "value": "Ready "},
                        {
                            "kind": "element",
                            "name": "strong",
                            "children": [{"kind": "text", "value": "now"}]
                        },
                        {"kind": "text", "value": "."}
                    ]
                }
            ]),
            None,
            Vec::new(),
            "test-converter-formatted-cem-tree",
            Some("output"),
            Some("converter.cemt"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(execution.format_execution, None);
        assert_eq!(
            execution.color_execution,
            Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: CEM_TREE_COLOR_CEMT_ADAPTER_ID.to_owned(),
                function_name: "acme.showcase.color-tree".to_owned(),
                body_function_name: Some("acme.showcase.color-tree".to_owned()),
                fallback_function_name: None,
            })
        );
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted converter CEM tree is retained");
        assert_eq!(formatted.value["kind"], "cem-tree");
        assert_eq!(
            formatted.value["formatterProfile"],
            "acme.showcase.format-tree"
        );
        assert_eq!(formatted.value["nodes"][0]["sourceMap"], Value::Null);
        assert_eq!(
            formatted.value["nodes"][0]["attributes"],
            Value::Array(vec![])
        );
        assert!(formatted.value["formatNodes"]
            .as_array()
            .expect("format nodes")
            .iter()
            .any(|node| node.get("name").and_then(Value::as_str) == Some("converter-cemt")));

        let output = execution
            .output
            .as_ref()
            .and_then(Value::as_str)
            .expect("writer output text");
        assert!(output.contains("<article"));
        assert!(output.contains("cem-color-syntax-name"));
        assert!(output.contains("<strong class=\"cem-color cem-color-syntax-keyword\""));
        assert!(output.contains(">now<"));
    }

    #[test]
    fn conversion_output_pipeline_rejects_claimed_formatted_cem_tree_without_formatter_metadata() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };

        let execution = execute_conversion_output_pipeline_from_formatted_cem_tree_with_environment(
            &environment,
            &direct_html_output_pipeline(),
            serde_json::json!({
                "kind": "cem-tree",
                "nodes": [{
                    "kind": "element",
                    "name": "main",
                    "children": [{"kind": "text", "value": "Ready"}]
                }]
            }),
            None,
            Vec::new(),
            "test-claimed-formatted-tree",
            Some("output"),
            Some("converter.cemt"),
        );

        assert!(execution.output.is_none());
        assert!(execution.formatted_cem_tree.is_none());
        assert_eq!(execution.diagnostics.len(), 1);
        let diagnostic = &execution.diagnostics[0];
        assert_eq!(diagnostic.code, CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains(
            "converter `test-claimed-formatted-tree` could not execute CEMT output pipeline"
        ));
        assert!(diagnostic
            .message
            .contains("claims formatted CEM tree but omits required formatter metadata"));
        assert!(diagnostic.message.contains("formatterProfile"));
    }

    #[test]
    fn cemt_formatter_coloring_pipeline_package_fixture_uses_manifest_artifacts() {
        let fixture = cemt_formatter_coloring_pipeline_package_fixture_source()
            .expect("manifest CEMT pipeline fixture");

        assert!(fixture.contains(r#"@source="schema-packages/cem-ml/v1/package.cem""#));
        assert!(fixture.contains(r#"@formatter="acme.showcase.format-tree""#));
        assert!(fixture.contains(r#"@colorizer="acme.showcase.color-tree""#));
        assert!(fixture.contains(r#"@color-profile="classes""#));
        assert!(fixture.contains(r#"@value="formatted tree before writer""#));
        assert!(fixture.contains(r#"@value="colored tree before writer""#));
        assert!(fixture.contains(r#"@stage="after-color""#));
        assert!(fixture.contains(r#"@value="writer consumes colored CEM tree""#));
        assert!(fixture.contains(r#"@colorizer-role="colorizer.queued-edit""#));
        assert!(fixture.contains(r#"@value="queued edit replay before writer""#));
        assert!(fixture.contains(r#"@value="cem-color cem-color-syntax-keyword""#));
    }

    #[test]
    fn cem_tree_output_templates_are_schema_package_assets() {
        let target = TransformTemplateEncodingTarget::new(
            CEM_ML_CONTENT_TYPE,
            CEM_ML_SCHEMA_URI,
            "cem-tree",
        );
        let formatter_profile = "compact";
        let color_profile = "classes";
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let package_id = conversion_package_id_for_encoding_target(&schema_registry, &target)
            .expect("CEM tree target package");
        assert_eq!(package_id, "cem-ml");

        let registry = ConversionRegistry::with_builtin_converters();
        let formatter = registry
            .select_package_artifact_for_output_stage(
                &package_id,
                "formatter",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                Some("cem.format-tree"),
                "cem.format-tree",
                Some(formatter_profile),
                None,
            )
            .expect("CEM tree formatter selector")
            .expect("CEM tree formatter package artifact");
        let colorizer = registry
            .select_package_artifact_for_output_stage(
                &package_id,
                "colorizer",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                Some("cem.color-tree"),
                "cem.color-tree",
                None,
                Some(color_profile),
            )
            .expect("CEM tree colorizer selector")
            .expect("CEM tree colorizer package artifact");
        let no_colorizer = registry
            .select_package_artifact_for_output_stage(
                &package_id,
                "colorizer",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                Some("cem.color-tree"),
                "cem.color-tree",
                None,
                Some("none"),
            )
            .expect("CEM tree no-color selector")
            .expect("CEM tree no-color package artifact");
        let showcase_formatter = registry
            .select_package_artifact_for_output_stage(
                &package_id,
                "formatter",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                Some("acme.showcase.format-tree"),
                "cem.format-tree",
                Some("acme.showcase.format-tree"),
                None,
            )
            .expect("showcase CEM tree formatter selector")
            .expect("showcase CEM tree formatter package artifact");
        let showcase_colorizer = registry
            .select_package_artifact_for_output_stage(
                &package_id,
                "colorizer",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                Some("acme.showcase.color-tree"),
                "cem.color-tree",
                None,
                Some(color_profile),
            )
            .expect("showcase CEM tree colorizer selector")
            .expect("showcase CEM tree colorizer package artifact");
        let default_showcase_formatter = registry
            .select_package_artifact_for_output_stage(
                &package_id,
                "formatter",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                None,
                "cem.format-tree",
                Some("acme.showcase.format-tree"),
                None,
            )
            .expect("showcase profile default selector")
            .expect("showcase profile default formatter package artifact");
        let default_colorizer = registry
            .select_package_artifact_for_output_stage(
                &package_id,
                "colorizer",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                None,
                "cem.color-tree",
                None,
                Some(color_profile),
            )
            .expect("color profile default selector")
            .expect("color profile default package artifact");
        let canonical_formatter_helper = registry
            .package_artifacts()
            .find(|artifact| {
                artifact.package_id == package_id
                    && artifact.kind == CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND
                    && artifact.path
                        == "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt"
            })
            .expect("canonical CEM tree formatter helper artifact");
        let canonical_colorizer_helper = registry
            .package_artifacts()
            .find(|artifact| {
                artifact.package_id == package_id
                    && artifact.kind == CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND
                    && artifact.path
                        == "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt"
            })
            .expect("canonical CEM tree colorizer helper artifact");
        let formatter_helper = registry
            .package_artifacts()
            .find(|artifact| {
                artifact.package_id == package_id
                    && artifact.kind == CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND
                    && artifact.path == "schema-packages/cem-ml/v1/formatters/cem-tree-helpers.cemt"
            })
            .expect("CEM tree formatter helper artifact");
        let colorizer_helper = registry
            .package_artifacts()
            .find(|artifact| {
                artifact.package_id == package_id
                    && artifact.kind == CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND
                    && artifact.path == "schema-packages/cem-ml/v1/colorizers/cem-tree-helpers.cemt"
            })
            .expect("CEM tree colorizer helper artifact");

        assert_eq!(
            formatter.path.as_str(),
            "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt"
        );
        assert_eq!(
            colorizer.path.as_str(),
            "schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt"
        );
        assert_eq!(no_colorizer.path, colorizer.path);
        assert_eq!(
            showcase_formatter.path.as_str(),
            "schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt"
        );
        assert_eq!(
            showcase_colorizer.path.as_str(),
            "schema-packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt"
        );
        assert_eq!(default_showcase_formatter.path, showcase_formatter.path);
        assert_eq!(default_colorizer.path, colorizer.path);
        assert_eq!(
            canonical_formatter_helper.function_name.as_deref(),
            Some("cem.format-tree.apply-stage")
        );
        assert_eq!(
            canonical_formatter_helper.function_profile.as_deref(),
            Some("cem.format-tree")
        );
        assert_eq!(
            canonical_formatter_helper.formatter_profile.as_deref(),
            Some(formatter_profile)
        );
        assert_eq!(
            canonical_colorizer_helper.function_name.as_deref(),
            Some("cem.color-tree.apply-stage")
        );
        assert_eq!(
            canonical_colorizer_helper.function_profile.as_deref(),
            Some("css-custom-properties")
        );
        assert_eq!(canonical_colorizer_helper.color_profile.as_deref(), None);
        assert_eq!(
            formatter.target_content_type.as_deref(),
            Some(CEM_ML_CONTENT_TYPE)
        );
        assert_eq!(formatter.target_schema.as_deref(), Some(CEM_ML_SCHEMA_URI));
        assert_eq!(formatter.target_category.as_deref(), Some("cem-tree"));
        assert_eq!(formatter.function_name.as_deref(), Some("cem.format-tree"));
        assert_eq!(
            formatter.formatter_profile.as_deref(),
            Some(formatter_profile)
        );
        assert_eq!(
            colorizer.target_content_type.as_deref(),
            Some(CEM_ML_CONTENT_TYPE)
        );
        assert_eq!(colorizer.target_schema.as_deref(), Some(CEM_ML_SCHEMA_URI));
        assert_eq!(colorizer.target_category.as_deref(), Some("cem-tree"));
        assert_eq!(colorizer.function_name.as_deref(), Some("cem.color-tree"));
        assert_eq!(
            colorizer.function_profile.as_deref(),
            Some("css-custom-properties")
        );
        assert_eq!(colorizer.color_profile.as_deref(), Some(color_profile));
        assert_eq!(
            showcase_formatter.function_name.as_deref(),
            Some("acme.showcase.format-tree")
        );
        assert_eq!(
            showcase_formatter.formatter_profile.as_deref(),
            Some("acme.showcase.format-tree")
        );
        assert_eq!(
            showcase_colorizer.function_name.as_deref(),
            Some("acme.showcase.color-tree")
        );
        assert_eq!(
            showcase_colorizer.function_profile.as_deref(),
            Some(color_profile)
        );
        assert_eq!(
            formatter_helper.function_name.as_deref(),
            Some("cemml.cem-tree.format-tree-base")
        );
        assert_eq!(
            formatter_helper.formatter_profile.as_deref(),
            Some("acme.showcase.format-tree")
        );
        assert_eq!(
            colorizer_helper.function_name.as_deref(),
            Some("cemml.cem-tree.color-tree-base")
        );
        assert_eq!(
            colorizer_helper.function_profile.as_deref(),
            Some("classes")
        );
        assert_eq!(
            colorizer_helper.color_profile.as_deref(),
            Some(color_profile)
        );
        let formatter_source =
            builtin_schema_package_artifact_source(&formatter.package_id, &formatter.path)
                .expect("embedded formatter source");
        let colorizer_source =
            builtin_schema_package_artifact_source(&colorizer.package_id, &colorizer.path)
                .expect("embedded colorizer source");
        let canonical_formatter_helper_source = builtin_schema_package_artifact_source(
            &canonical_formatter_helper.package_id,
            &canonical_formatter_helper.path,
        )
        .expect("embedded canonical formatter helper source");
        let canonical_colorizer_helper_source = builtin_schema_package_artifact_source(
            &canonical_colorizer_helper.package_id,
            &canonical_colorizer_helper.path,
        )
        .expect("embedded canonical colorizer helper source");
        let showcase_formatter_source = builtin_schema_package_artifact_source(
            &showcase_formatter.package_id,
            &showcase_formatter.path,
        )
        .expect("embedded showcase formatter source");
        let showcase_colorizer_source = builtin_schema_package_artifact_source(
            &showcase_colorizer.package_id,
            &showcase_colorizer.path,
        )
        .expect("embedded showcase colorizer source");
        let formatter_helper_source = builtin_schema_package_artifact_source(
            &formatter_helper.package_id,
            &formatter_helper.path,
        )
        .expect("embedded formatter helper source");
        let colorizer_helper_source = builtin_schema_package_artifact_source(
            &colorizer_helper.package_id,
            &colorizer_helper.path,
        )
        .expect("embedded colorizer helper source");
        assert!(formatter_source.source.contains("{format-function"));
        assert!(colorizer_source.source.contains("{color-function"));
        assert!(canonical_formatter_helper_source
            .source
            .contains(r#"@name="cem.format-tree.apply-stage""#));
        assert!(canonical_colorizer_helper_source
            .source
            .contains(r#"@name="cem.color-tree.apply-stage""#));
        assert!(showcase_formatter_source
            .source
            .contains(r#"@extends="cem.format-tree""#));
        assert!(showcase_colorizer_source
            .source
            .contains(r#"@extends="cem.color-tree""#));
        assert!(formatter_helper_source
            .source
            .contains(r#"@name="cemml.cem-tree.format-tree-base""#));
        assert!(colorizer_helper_source
            .source
            .contains(r#"@name="cemml.cem-tree.color-tree-base""#));
        assert_eq!(
            cem_tree_format_cemt_stage(&target, Some(formatter_profile))
                .unwrap()
                .template_uri,
            formatter.path
        );
        assert_eq!(
            cem_tree_color_cemt_stage(&target, Some(color_profile))
                .unwrap()
                .template_uri,
            colorizer.path
        );
        for profile in ["compact", "pretty", "tabular"] {
            let stage = cem_tree_format_cemt_stage(&target, Some(profile))
                .expect("baseline formatter profile resolves");
            assert_eq!(
                stage.template_uri,
                "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt"
            );
            assert_eq!(stage.stage_profile.as_deref(), Some(profile));
        }
        for profile in ["terminal", "html", "md"] {
            let stage = cem_tree_color_cemt_stage(&target, Some(profile))
                .expect("baseline colorizer profile resolves");
            assert_eq!(
                stage.template_uri,
                "schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt"
            );
            assert_eq!(stage.stage_profile.as_deref(), Some(profile));
        }
        for source in [formatter_source.source, colorizer_source.source] {
            assert!(source.contains(r#"@content-type="application/cem""#));
            assert!(source.contains(r#"@schema="https://cem.dev/ns/cem-ml/1""#));
        }
    }

    #[test]
    fn cem_ql_output_templates_are_schema_package_assets() {
        let target = TransformTemplateEncodingTarget::new(
            CEM_QL_CONTENT_TYPE,
            CEM_QL_SCHEMA_URI,
            "cem-tree",
        );
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let package_id = conversion_package_id_for_encoding_target(&schema_registry, &target)
            .expect("CEM-QL CEM tree target package");
        assert_eq!(package_id, "cem-ql");

        for profile in ["compact", "pretty", "tabular"] {
            let artifact = conversion_registry
                .select_package_artifact_for_output_stage(
                    &package_id,
                    "formatter",
                    Some(CEM_TRANSFORM_CONTENT_TYPE),
                    Some(CEM_TRANSFORM_SCHEMA_URI),
                    &target,
                    Some("cem-ql.format-tree"),
                    "cem.format-tree",
                    Some(profile),
                    None,
                )
                .expect("CEM-QL formatter selector")
                .unwrap_or_else(|| panic!("CEM-QL formatter profile `{profile}` resolves"));
            assert_eq!(
                artifact.path,
                "schema-packages/cem-ql/v1/formatters/cem-ql-format-tree.cemt"
            );
        }
        for profile in ["terminal", "html", "md", "none"] {
            let artifact = conversion_registry
                .select_package_artifact_for_output_stage(
                    &package_id,
                    "colorizer",
                    Some(CEM_TRANSFORM_CONTENT_TYPE),
                    Some(CEM_TRANSFORM_SCHEMA_URI),
                    &target,
                    Some("cem-ql.color-tree"),
                    "cem.color-tree",
                    None,
                    Some(profile),
                )
                .expect("CEM-QL colorizer selector")
                .unwrap_or_else(|| panic!("CEM-QL colorizer profile `{profile}` resolves"));
            assert_eq!(
                artifact.path,
                "schema-packages/cem-ql/v1/colorizers/cem-ql-color-tree.cemt"
            );
        }

        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let cemt_options = TransformTemplateEncodeOptions {
            formatter: Some("cem-ql.format-tree".to_owned()),
            formatter_profile: Some("tabular".to_owned()),
            colorizer: Some("cem-ql.color-tree".to_owned()),
            color_profile: Some("html".to_owned()),
            mode: TransformTemplateEncodedArtifactMode::Document,
            source_map_policy: TransformTemplateSourceMapPolicy::Generated,
            ..TransformTemplateEncodeOptions::default()
        };
        let mut cemt_context =
            TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                &target,
                Some(TransformTemplateOutputProducedKind::CemTree),
            );
        cemt_context.formatter_profile = Some("tabular".to_owned());
        cemt_context.color_profile = Some("html".to_owned());
        let writer_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
            &target,
            Some(TransformTemplateOutputProducedKind::Text),
        );
        let pipeline = ConversionOutputPipeline {
            stages: vec![
                ConversionOutputPipelineStage::Transform,
                ConversionOutputPipelineStage::Format,
                ConversionOutputPipelineStage::Color,
                ConversionOutputPipelineStage::Writer,
            ],
            cemt_target: target.clone(),
            cemt_options: cemt_options.clone(),
            cemt_insertion_context: cemt_context,
            cemt_produces: TransformTemplateOutputProducedKind::CemTree,
            writer_insertion_context: writer_context,
            writer_produces: TransformTemplateOutputProducedKind::Text,
        };
        let functions = conversion_cem_tree_output_function_registry(&environment, &pipeline);
        assert!(
            functions.functions().iter().any(|function| {
                function.kind == TransformTemplateOutputFunctionKind::Format
                    && function.name == "cem-ql.format-tree"
                    && function.profile.as_deref() == Some("tabular")
                    && function.subject == "cem-ast-node"
                    && content_type_essence(&function.content_type) == CEM_QL_CONTENT_TYPE
                    && function.schema == CEM_QL_SCHEMA_URI
                    && function.category == "cem-tree"
            }),
            "registered functions: {:?}",
            functions
                .functions()
                .iter()
                .map(|function| (
                    function.kind,
                    function.name.as_str(),
                    function.profile.as_deref(),
                    function.subject.as_str(),
                    function.content_type.as_str(),
                    function.schema.as_str(),
                    function.category.as_str()
                ))
                .collect::<Vec<_>>()
        );
        let request = TransformTemplateEncodeBindingRequest::new(
            serde_json::json!({"kind": "cem-ql-source", "tokens": []}),
            target.clone(),
        )
        .with_subject_type("cem-ast-node")
        .with_options(cemt_options);
        let binding = functions
            .resolve_format_binding(&request, &BTreeSet::new())
            .expect("CEM-QL package formatter binding resolves");
        assert_eq!(binding.function.name, "cem-ql.format-tree");
        assert_eq!(binding.function.profile.as_deref(), Some("tabular"));

        let subject = serde_json::json!({
            "kind": "cem-ql-source",
            "tokens": [
                {
                    "kind": "cem-ql.keyword",
                    "text": "module",
                    "role": "syntax.keyword",
                    "value": { "tokenKind": "Module" }
                },
                {
                    "kind": "cem-ql.whitespace",
                    "text": " ",
                    "role": "source.whitespace",
                    "value": { "tokenKind": "Whitespace" }
                },
                {
                    "kind": "cem-ql.string",
                    "text": "\"https://example.test/q\"",
                    "role": "syntax.string",
                    "value": { "tokenKind": "StringLit" }
                }
            ]
        });
        for profile in ["compact", "pretty", "tabular"] {
            let cemt_options = TransformTemplateEncodeOptions {
                formatter: Some("cem-ql.format-tree".to_owned()),
                formatter_profile: Some(profile.to_owned()),
                colorizer: Some("cem-ql.color-tree".to_owned()),
                color_profile: Some("none".to_owned()),
                mode: TransformTemplateEncodedArtifactMode::Document,
                source_map_policy: TransformTemplateSourceMapPolicy::Generated,
                ..TransformTemplateEncodeOptions::default()
            };
            let mut cemt_context =
                TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                    &target,
                    Some(TransformTemplateOutputProducedKind::CemTree),
                );
            cemt_context.formatter_profile = Some(profile.to_owned());
            cemt_context.color_profile = Some("none".to_owned());
            cemt_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
            cemt_context.canonical = Some(false);
            cemt_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
            let mut writer_context =
                TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                    &target,
                    Some(TransformTemplateOutputProducedKind::Text),
                );
            writer_context.formatter_profile = Some(profile.to_owned());
            writer_context.color_profile = Some("none".to_owned());
            writer_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
            writer_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
            let pipeline = ConversionOutputPipeline {
                stages: vec![
                    ConversionOutputPipelineStage::Transform,
                    ConversionOutputPipelineStage::Format,
                    ConversionOutputPipelineStage::Color,
                    ConversionOutputPipelineStage::Writer,
                ],
                cemt_target: target.clone(),
                cemt_options,
                cemt_insertion_context: cemt_context,
                cemt_produces: TransformTemplateOutputProducedKind::CemTree,
                writer_insertion_context: writer_context,
                writer_produces: TransformTemplateOutputProducedKind::Text,
            };
            let execution = execute_conversion_output_pipeline_with_environment(
                &environment,
                &pipeline,
                subject.clone(),
                None,
                Vec::new(),
                "cem-ql-test-output",
                Some("cem-ql"),
                None,
            );
            assert!(
                execution.diagnostics.is_empty(),
                "{:?}",
                execution.diagnostics
            );
            assert_eq!(
                execution
                    .formatted_cem_tree
                    .as_ref()
                    .map(|artifact| &artifact.value["formatterProfile"]),
                Some(&Value::String(profile.to_owned()))
            );
            assert_eq!(
                execution
                    .colored_cem_tree
                    .as_ref()
                    .map(|artifact| &artifact.value["colorProfile"]),
                Some(&Value::String("none".to_owned()))
            );
            assert_eq!(
                execution.output.as_ref().and_then(Value::as_str),
                Some("module \"https://example.test/q\"")
            );
        }
    }

    #[test]
    fn cemt_output_stage_profile_default_reports_ambiguous_package_artifacts() {
        let target = TransformTemplateEncodingTarget::new(
            CEM_ML_CONTENT_TYPE,
            CEM_ML_SCHEMA_URI,
            "cem-tree",
        );
        let mut registry = ConversionRegistry::new();
        let artifact = |kind: &str,
                        path: &str,
                        function_name: &str,
                        formatter_profile: Option<&str>,
                        color_profile: Option<&str>| {
            ConversionPackageArtifactDescriptor {
                package_id: "cem-ml".to_owned(),
                kind: kind.to_owned(),
                path: path.to_owned(),
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
                target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
                target_category: Some("cem-tree".to_owned()),
                function_name: Some(function_name.to_owned()),
                function_profile: None,
                formatter_profile: formatter_profile.map(str::to_owned),
                color_profile: color_profile.map(str::to_owned),
                generated: false,
            }
        };

        registry.register_package_artifact(artifact(
            "formatter",
            "schema-packages/cem-ml/v1/formatters/acme-a.cemt",
            "acme.format-a",
            Some("acme.profile"),
            None,
        ));
        registry.register_package_artifact(artifact(
            "formatter",
            "schema-packages/cem-ml/v1/formatters/acme-b.cemt",
            "acme.format-b",
            Some("acme.profile"),
            None,
        ));
        registry.register_package_artifact(artifact(
            "colorizer",
            "schema-packages/cem-ml/v1/colorizers/acme-a.cemt",
            "acme.color-a",
            None,
            Some("classes"),
        ));
        registry.register_package_artifact(artifact(
            "colorizer",
            "schema-packages/cem-ml/v1/colorizers/acme-b.cemt",
            "acme.color-b",
            None,
            Some("classes"),
        ));

        let formatter_error = registry
            .select_package_artifact_for_output_stage(
                "cem-ml",
                "formatter",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                None,
                "cem.format-tree",
                Some("acme.profile"),
                None,
            )
            .expect_err("ambiguous formatter profile default is rejected")
            .to_string();
        assert!(formatter_error.contains("multiple `formatter` CEMT artifacts"));
        assert!(formatter_error.contains("profile `acme.profile`"));
        assert!(formatter_error.contains("acme.format-a"));
        assert!(formatter_error.contains("acme.format-b"));

        let colorizer_error = registry
            .select_package_artifact_for_output_stage(
                "cem-ml",
                "colorizer",
                Some(CEM_TRANSFORM_CONTENT_TYPE),
                Some(CEM_TRANSFORM_SCHEMA_URI),
                &target,
                None,
                "cem.color-tree",
                None,
                Some("classes"),
            )
            .expect_err("ambiguous colorizer profile default is rejected")
            .to_string();
        assert!(colorizer_error.contains("multiple `colorizer` CEMT artifacts"));
        assert!(colorizer_error.contains("profile `classes`"));
        assert!(colorizer_error.contains("acme.color-a"));
        assert!(colorizer_error.contains("acme.color-b"));
    }

    #[test]
    fn cemt_output_asset_package_resolution_rejects_target_identity_mismatch() {
        let target =
            TransformTemplateEncodingTarget::new(HTML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, "cem-tree");

        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let error = conversion_package_id_for_encoding_target(&schema_registry, &target)
            .expect_err("HTML content type is not owned by the CEM-ML schema");

        assert!(error.contains("is not owned by schema"));
        assert!(error.contains(HTML_CONTENT_TYPE));
        assert!(error.contains(CEM_ML_SCHEMA_URI));
    }

    #[test]
    fn cemt_output_asset_metadata_must_match_referenced_cemt_function() {
        let package = load_builtin_schema_package(CEM_ML_SCHEMA_URI).expect("CEM-ML package");
        let manifest = package.manifest_source.replace(
            r#"@function-name="cem.format-tree""#,
            r#"@function-name="cem.missing-tree""#,
        );
        let package = BuiltinSchemaPackage {
            manifest_source: Box::leak(manifest.into_boxed_str()),
            ..package
        };

        let error = conversion_package_artifacts_from_schema_package(&package)
            .expect_err("wrong CEMT function name is rejected");

        match error {
            ConversionManifestError::ArtifactContract { path, message, .. } => {
                assert_eq!(
                    path,
                    "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt"
                );
                assert!(message.contains("no CEMT output function named `cem.missing-tree`"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cemt_output_asset_target_metadata_must_match_referenced_cemt_function() {
        let package = load_builtin_schema_package(CEM_ML_SCHEMA_URI).expect("CEM-ML package");
        let manifest = package.manifest_source.replacen(
            r#"@target-category="cem-tree""#,
            r#"@target-category="wrong-tree""#,
            1,
        );
        let package = BuiltinSchemaPackage {
            manifest_source: Box::leak(manifest.into_boxed_str()),
            ..package
        };

        let error = conversion_package_artifacts_from_schema_package(&package)
            .expect_err("wrong CEMT target category is rejected");

        match error {
            ConversionManifestError::ArtifactContract { path, message, .. } => {
                assert_eq!(
                    path,
                    "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt"
                );
                assert!(message.contains("target category metadata expected `wrong-tree`"));
                assert!(message.contains("CEMT declares `cem-tree`"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cemt_formatter_artifact_body_must_build_formatted_tree_metadata() {
        let artifact = ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "formatter".to_owned(),
            path: "schema-packages/cem-ml/v1/formatters/bare-tree.cemt".to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("acme.bare.format-tree".to_owned()),
            function_profile: None,
            formatter_profile: Some("acme.bare.format-tree".to_owned()),
            color_profile: None,
            generated: false,
        };
        let source = r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="acme.bare.format-tree"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="any" @required=true}
        {body | {$ { kind: "cem-tree", nodes: [$subject] } } }
    }
}
"#;
        let parse_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: artifact.path.clone(),
                    bytes: source.as_bytes().to_vec(),
                    identity: Some(FormatIdentity {
                        content_type: artifact.content_type.clone(),
                        schema: artifact.schema.clone(),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });
        assert!(
            parse_response.diagnostics.is_empty(),
            "{:?}",
            parse_response.diagnostics
        );
        let function = parse_response
            .module_options
            .output_functions
            .iter()
            .find(|function| function.name == "acme.bare.format-tree")
            .expect("bare formatter function");

        let error = validate_conversion_package_artifact_cem_tree_stage_metadata_contract(
            &artifact,
            function,
            &parse_response.module_options,
            &[],
        )
        .expect_err("bare formatter body is rejected");

        match error {
            ConversionManifestError::ArtifactContract { path, message, .. } => {
                assert_eq!(path, artifact.path);
                assert!(message.contains("formatted CEM tree metadata"));
                assert!(message.contains("formatterProfile"));
                assert!(message.contains("formatNodes"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cemt_colorizer_artifact_body_must_build_colored_tree_metadata() {
        let artifact = ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "colorizer".to_owned(),
            path: "schema-packages/cem-ml/v1/colorizers/bare-tree.cemt".to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("acme.bare.color-tree".to_owned()),
            function_profile: Some("classes".to_owned()),
            formatter_profile: None,
            color_profile: Some("classes".to_owned()),
            generated: false,
        };
        let source = r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {color-function
        @name="acme.bare.color-tree"
        @category="cem-tree"
        @subject="cem-tree"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="classes"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {body | {$ $subject } }
    }
}
"#;
        let parse_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: artifact.path.clone(),
                    bytes: source.as_bytes().to_vec(),
                    identity: Some(FormatIdentity {
                        content_type: artifact.content_type.clone(),
                        schema: artifact.schema.clone(),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });
        assert!(
            parse_response.diagnostics.is_empty(),
            "{:?}",
            parse_response.diagnostics
        );
        let function = parse_response
            .module_options
            .output_functions
            .iter()
            .find(|function| function.name == "acme.bare.color-tree")
            .expect("bare colorizer function");

        let error = validate_conversion_package_artifact_cem_tree_stage_metadata_contract(
            &artifact,
            function,
            &parse_response.module_options,
            &[],
        )
        .expect_err("bare colorizer body is rejected");

        match error {
            ConversionManifestError::ArtifactContract { path, message, .. } => {
                assert_eq!(path, artifact.path);
                assert!(message.contains("colored CEM tree metadata"));
                assert!(message.contains("colorProfile"));
                assert!(message.contains("colorNodes"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn cemt_output_asset_package_resolution_rejects_profile_mismatch() {
        let target = TransformTemplateEncodingTarget::new(
            CEM_ML_CONTENT_TYPE,
            CEM_ML_SCHEMA_URI,
            "cem-tree",
        );

        let error = cem_tree_color_cemt_stage(&target, Some("terminal.truecolor"))
            .expect_err("unknown CEM tree color profile has no schema package artifact");

        assert!(error.contains("does not declare a `colorizer` CEMT artifact"));
        assert!(error.contains("profile `terminal.truecolor`"));
    }

    #[test]
    fn builtin_cem_tree_formatter_template_delegates_to_helper_artifact() {
        let target = TransformTemplateEncodingTarget::new(
            CEM_ML_CONTENT_TYPE,
            CEM_ML_SCHEMA_URI,
            "cem-tree",
        );
        let stage =
            cem_tree_format_cemt_stage(&target, Some("compact")).expect("CEM tree formatter stage");
        let response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: stage.template_uri.to_owned(),
                    bytes: stage.template_bytes.clone(),
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(response.module_options.encode_expressions.is_empty());
        let formatter = response
            .module_options
            .output_functions
            .iter()
            .find(|function| function.name == "cem.format-tree")
            .expect("built-in CEM tree formatter declaration");
        assert_eq!(
            formatter
                .params
                .iter()
                .find(|param| param.name == "subject")
                .map(|param| param.value_type.as_contract_name()),
            Some("any")
        );
        assert_eq!(
            formatter.body_expression.as_deref(),
            Some(r#"call("cem.format-tree.apply-stage", { subject: $subject })"#)
        );
        let helper_source = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt",
        )
        .expect("CEM tree formatter helper source");
        let helper_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: helper_source.path.to_owned(),
                    bytes: helper_source.source.as_bytes().to_vec(),
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });
        assert!(
            helper_response.diagnostics.is_empty(),
            "{:?}",
            helper_response.diagnostics
        );
        let helper = helper_response
            .module_options
            .output_functions
            .iter()
            .find(|function| function.name == "cem.format-tree.apply-stage")
            .expect("CEM tree formatter helper declaration");
        let body = helper
            .body_expression
            .as_deref()
            .expect("formatter helper body expression");
        assert!(body.contains("appendFormatNode("));
        assert!(body.contains("applyEdits("));
        assert!(body.contains(r#"call("cem.format-tree.build-nodes""#));
        assert!(body.contains(r#"call("cem.format-tree.build-envelope""#));
        let build_nodes = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.format-tree.build-nodes")
            .expect("CEM tree node builder helper declaration");
        assert_eq!(
            build_nodes.return_type,
            crate::transform_template::TransformTemplateModuleParamType::Array
        );
        assert!(build_nodes
            .body_expression
            .as_deref()
            .is_some_and(|body| body
                .contains(r#"call("cem.format-tree.format-inter-node-whitespace""#)
                && body.contains(r#"call("cem.format-tree.build-node-list""#)
                && !body.contains("cem.format-tree.nodes")
                && !body.contains("cem.format-tree.inter-node-whitespace")));
        let format_inter_node_whitespace = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.format-tree.format-inter-node-whitespace")
            .expect("CEM tree inter-node whitespace helper declaration");
        assert_eq!(
            format_inter_node_whitespace.return_type,
            crate::transform_template::TransformTemplateModuleParamType::Array
        );
        assert!(format_inter_node_whitespace
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains("fold($subject")
                && body.contains("last($acc)")
                && body.contains("formatter.line-ending")));
        assert!(helper_response
            .module_options
            .functions
            .iter()
            .any(|function| function.name == "cem.format-tree.build-node-list"));
        assert!(helper_response
            .module_options
            .functions
            .iter()
            .any(|function| function.name == "cem.format-tree.format-node"));
        let format_children = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.format-tree.format-children")
            .expect("CEM tree child formatter helper declaration");
        assert!(format_children
            .body_expression
            .as_deref()
            .is_some_and(
                |body| body.contains(r#"call("cem.format-tree.format-block-children""#)
                    && !body.contains("cem.format-tree.block-children")
            ));
        let format_block_children = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.format-tree.format-block-children")
            .expect("CEM tree block child formatter helper declaration");
        assert_eq!(
            format_block_children.return_type,
            crate::transform_template::TransformTemplateModuleParamType::Array
        );
        assert!(format_block_children
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains("fold($subject")
                && body.contains("repeat($indent, $depth)")
                && body.contains("formatter.indent")
                && body.contains("formatter.line-ending")));
        let formatter_whitespace = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.format-tree.formatter-whitespace")
            .expect("CEM tree formatter whitespace helper declaration");
        assert!(formatter_whitespace
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains(r#"sourceMap($subject, "cem.format-tree")"#)));
        assert!(helper_response
            .module_options
            .functions
            .iter()
            .any(|function| function.name == "cem.format-tree.node-child-layout"));
        let build_content_boundary = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.format-tree.build-content-boundary")
            .expect("CEM tree content boundary helper declaration");
        assert_eq!(
            build_content_boundary.return_type,
            crate::transform_template::TransformTemplateModuleParamType::Array
        );
        assert!(build_content_boundary
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains("formatter.content-boundary")
                && body.contains("formatter.boundary-spacing")
                && body.contains("formatter.line-ending")
                && !body.contains("cem.format-tree.content-boundary")));
        let build_envelope = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.format-tree.build-envelope")
            .expect("CEM tree envelope builder helper declaration");
        assert_eq!(
            build_envelope.return_type,
            crate::transform_template::TransformTemplateModuleParamType::Object
        );
        assert!(build_envelope
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains(r#"kind: "cem-tree""#)
                && body.contains("mode: $mode")
                && body.contains("formatterProfile: $formatterProfile")
                && body.contains(r#"call("cem.format-tree.add-format-nodes""#)
                && !body.contains("cem.format-tree.format-nodes")
                && !body.contains("cem.format-tree.envelope")));
        assert!(helper_response
            .module_options
            .functions
            .iter()
            .any(|function| function.name == "cem.format-tree.add-format-nodes"));
    }

    #[test]
    fn builtin_cem_tree_colorizer_template_delegates_to_helper_artifact() {
        let target = TransformTemplateEncodingTarget::new(
            CEM_ML_CONTENT_TYPE,
            CEM_ML_SCHEMA_URI,
            "cem-tree",
        );
        let stage =
            cem_tree_color_cemt_stage(&target, Some("classes")).expect("CEM tree colorizer stage");
        let response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: stage.template_uri.to_owned(),
                    bytes: stage.template_bytes.clone(),
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(response.module_options.encode_expressions.is_empty());
        let colorizer = response
            .module_options
            .output_functions
            .iter()
            .find(|function| function.name == "cem.color-tree")
            .expect("built-in CEM tree colorizer declaration");
        assert_eq!(
            colorizer
                .params
                .iter()
                .find(|param| param.name == "subject")
                .map(|param| param.value_type.as_contract_name()),
            Some("object")
        );
        assert_eq!(
            colorizer.body_expression.as_deref(),
            Some(r#"call("cem.color-tree.apply-stage", { subject: $subject })"#)
        );
        let helper_source = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt",
        )
        .expect("CEM tree colorizer helper source");
        let helper_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: helper_source.path.to_owned(),
                    bytes: helper_source.source.as_bytes().to_vec(),
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });
        assert!(
            helper_response.diagnostics.is_empty(),
            "{:?}",
            helper_response.diagnostics
        );
        let helper = helper_response
            .module_options
            .output_functions
            .iter()
            .find(|function| function.name == "cem.color-tree.apply-stage")
            .expect("CEM tree colorizer helper declaration");
        let body = helper
            .body_expression
            .as_deref()
            .expect("colorizer helper body expression");
        assert!(body.contains(r#"call("cem.color-tree.apply-profile""#));
        assert!(!body.contains("call(cem.color-tree.apply"));
        let apply_profile = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.color-tree.apply-profile")
            .expect("CEM tree colorizer apply-profile helper declaration");
        assert!(apply_profile
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains("appendColorNode(")
                && body.contains("applyEdits(")
                && body.contains(r#"call("cem.color-tree.color-tree""#)));
        assert!(helper_response
            .module_options
            .functions
            .iter()
            .any(|function| function.name == "cem.color-tree.color-node"));
        let color_nodes = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.color-tree.color-nodes")
            .expect("CEM tree colorizer color-nodes helper declaration");
        assert!(color_nodes
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains("source: $subject")
                && body.contains("cem.color-tree.color-marker")
                && body.contains("cem.color-tree.color-decision")));
        let generated_node = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.color-tree.generated-node")
            .expect("CEM tree colorizer generated node helper declaration");
        assert_eq!(
            generated_node.return_type,
            crate::transform_template::TransformTemplateModuleParamType::Object
        );
        assert!(generated_node
            .body_expression
            .as_deref()
            .is_some_and(
                |body| body.contains(r#"sourceMap($source, "cem.color-tree")"#)
                    && body.contains(r#"set($subject, "sourceMap""#)
            ));
        assert!(helper_response
            .module_options
            .functions
            .iter()
            .any(|function| function.name == "cem.color-tree.writer-attribute-nodes"));
        let writer_attribute = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.color-tree.writer-attribute")
            .expect("CEM tree colorizer writer attribute helper declaration");
        assert!(writer_attribute
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains("cem.color-tree.generated-node")
                && body.contains("colorizer.writer-attribute")));
        let wrapper_nodes = helper_response
            .module_options
            .functions
            .iter()
            .find(|function| function.name == "cem.color-tree.wrapper-nodes")
            .expect("CEM tree colorizer wrapper nodes helper declaration");
        assert!(wrapper_nodes
            .body_expression
            .as_deref()
            .is_some_and(|body| body.contains("cem.color-tree.generated-node")
                && body.contains("colorizer.text-wrapper")
                && body.contains("colorizer.wrapped-role")));
    }

    #[test]
    fn cemt_output_stage_executes_direct_formatter_body_expression() {
        let stage = CemTreeCemtOutputStage {
            adapter_id: "cem-tree-format-direct-cemt",
            package_id: "cem-ml".to_owned(),
            target: TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
            stage_profile: Some("compact".to_owned()),
            template_uri: "builtin:cem.format-tree.direct.cemt".to_owned(),
            template_bytes: r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="cem.format-tree"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {body |
            {$ {
                kind: "cem-tree",
                contentType: "application/cem",
                schema: "https://cem.dev/ns/cem-ml/1",
                category: "cem-tree",
                mode: "document",
                canonical: true,
                formatterProfile: "compact",
                formatNodes: [
                    {
                        kind: "format-marker",
                        name: "cem.format-tree",
                        formatterRole: "formatter.boundary",
                        formatterProfile: "compact"
                    },
                    {
                        kind: "format-decision",
                        name: "layout",
                        formatterRole: "formatter.layout",
                        value: "direct-cemt",
                        formatterProfile: "compact"
                    }
                ],
                nodes: [{
                    kind: "element",
                    name: $subject.name,
                    children: map($subject.children, { kind: $item.kind, value: $item.value })
                }]
            } }
        }
    }
}
"#
            .as_bytes()
            .to_vec(),
            declaration_element: "{format-function",
            function_kind: TransformTemplateOutputFunctionKind::Format,
            function_name: "cem.format-tree".to_owned(),
            canonical_function_name: "cem.format-tree",
            role: "formatter",
        };
        let subject = serde_json::json!({
            "kind": "element",
            "name": "main",
            "children": [{"kind": "text", "value": "Ready"}]
        });
        let mut registry = TransformTemplateOutputFunctionRegistry::new();
        registry.register(conversion_cem_tree_format_function_descriptor("compact"));
        let request = TransformTemplateEncodeBindingRequest::new(
            subject.clone(),
            TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
        )
        .with_subject_type("cem-ast-node")
        .with_options(TransformTemplateEncodeOptions {
            formatter: Some("cem.format-tree".to_owned()),
            canonical: true,
            ..TransformTemplateEncodeOptions::default()
        });
        let binding = registry
            .resolve_format_binding(&request, &BTreeSet::new())
            .expect("direct formatter binding resolves");

        let (formatted, execution) =
            execute_test_conversion_cem_tree_output_stage(stage, &binding, &subject)
                .expect("direct formatter body runs");

        assert_eq!(
            execution,
            ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: "cem-tree-format-direct-cemt".to_owned(),
                function_name: "cem.format-tree".to_owned(),
                body_function_name: Some("cem.format-tree".to_owned()),
                fallback_function_name: None,
            }
        );
        assert_eq!(formatted["kind"], "cem-tree");
        assert_eq!(formatted["formatterProfile"], "compact");
        assert_eq!(formatted["formatNodes"][0]["name"], "cem.format-tree");
        assert_eq!(
            formatted["formatNodes"][1]["formatterRole"],
            "formatter.layout"
        );
        assert_eq!(formatted["nodes"][0]["name"], "main");
        assert_eq!(formatted["nodes"][0]["children"][0]["value"], "Ready");

        binding
            .artifact_from_value(formatted)
            .validate_insertion(
                &TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                    &request.target,
                    Some(TransformTemplateOutputProducedKind::CemTree),
                ),
            )
            .expect("direct formatted CEM tree validates");
    }

    #[test]
    fn cemt_output_stage_rejects_missing_direct_body_instead_of_fallback() {
        let stage = CemTreeCemtOutputStage {
            adapter_id: "cem-tree-format-no-body-cemt",
            package_id: "cem-ml".to_owned(),
            target: TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
            stage_profile: Some("compact".to_owned()),
            template_uri: "builtin:cem.format-tree.no-body.cemt".to_owned(),
            template_bytes: r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="cem.format-tree"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true}
}
"#
            .as_bytes()
            .to_vec(),
            declaration_element: "{format-function",
            function_kind: TransformTemplateOutputFunctionKind::Format,
            function_name: "cem.format-tree".to_owned(),
            canonical_function_name: "cem.format-tree",
            role: "formatter",
        };
        let subject = serde_json::json!({
            "kind": "element",
            "name": "main",
            "children": [{"kind": "text", "value": "Ready"}]
        });
        let mut registry = TransformTemplateOutputFunctionRegistry::new();
        registry.register(conversion_cem_tree_format_function_descriptor(
            "cem.format-tree",
        ));
        let request = TransformTemplateEncodeBindingRequest::new(
            subject.clone(),
            TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
        )
        .with_subject_type("cem-ast-node")
        .with_options(TransformTemplateEncodeOptions {
            formatter: Some("cem.format-tree".to_owned()),
            canonical: true,
            ..TransformTemplateEncodeOptions::default()
        });
        let binding = registry
            .resolve_format_binding(&request, &BTreeSet::new())
            .expect("formatter binding resolves");

        let error = execute_test_conversion_cem_tree_output_stage(stage, &binding, &subject)
            .expect_err("output stage without direct body is rejected");

        assert!(error.contains("CEMT formatter `cem.format-tree` requires a direct CEMT body"));
        assert!(!error.contains("fallback implementation"));
    }

    #[test]
    fn cemt_output_stage_rejects_legacy_encode_facade_body() {
        let stage = CemTreeCemtOutputStage {
            adapter_id: "cem-tree-format-encode-facade-cemt",
            package_id: "cem-ml".to_owned(),
            target: TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
            stage_profile: Some("compact".to_owned()),
            template_uri: "builtin:cem.format-tree.encode-facade.cemt".to_owned(),
            template_bytes: r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="cem.format-tree"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="any" @required=true}
        {body |
            {$ encode($subject, { contentType: "application/cem", schema: "https://cem.dev/ns/cem-ml/1", category: "cem-tree", subjectType: "cem-ast-node" }, { formatter: "cem.format-tree" }) }
        }
    }
}
"#
            .as_bytes()
            .to_vec(),
            declaration_element: "{format-function",
            function_kind: TransformTemplateOutputFunctionKind::Format,
            function_name: "cem.format-tree".to_owned(),
            canonical_function_name: "cem.format-tree",
            role: "formatter",
        };
        let subject = serde_json::json!({
            "kind": "element",
            "name": "main",
            "children": [{"kind": "text", "value": "Ready"}]
        });
        let mut registry = TransformTemplateOutputFunctionRegistry::new();
        registry.register(conversion_cem_tree_format_function_descriptor(
            "cem.format-tree",
        ));
        let request = TransformTemplateEncodeBindingRequest::new(
            subject.clone(),
            TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
        )
        .with_subject_type("cem-ast-node")
        .with_options(TransformTemplateEncodeOptions {
            formatter: Some("cem.format-tree".to_owned()),
            canonical: true,
            ..TransformTemplateEncodeOptions::default()
        });
        let binding = registry
            .resolve_format_binding(&request, &BTreeSet::new())
            .expect("formatter binding resolves");

        let error = execute_test_conversion_cem_tree_output_stage(stage, &binding, &subject)
            .expect_err("legacy encode facade body is rejected");

        assert!(error.contains("CEMT formatter `cem.format-tree` requires a direct CEMT body"));
        assert!(error.contains("encode(...) facade attempted to dispatch `cem.format-tree`"));
    }

    #[test]
    fn cemt_output_stage_rejects_alias_without_canonical_extends() {
        let stage = CemTreeCemtOutputStage {
            adapter_id: "cem-tree-format-alias-cemt",
            package_id: "cem-ml".to_owned(),
            target: TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
            stage_profile: Some("acme.format-tree".to_owned()),
            template_uri: "builtin:acme.format-tree.no-extends.cemt".to_owned(),
            template_bytes: r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="acme.format-tree"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="any" @required=true}
        {body | {$ $subject } }
    }
}
"#
            .as_bytes()
            .to_vec(),
            declaration_element: "{format-function",
            function_kind: TransformTemplateOutputFunctionKind::Format,
            function_name: "acme.format-tree".to_owned(),
            canonical_function_name: "cem.format-tree",
            role: "formatter",
        };
        let subject = serde_json::json!({
            "kind": "element",
            "name": "main",
            "children": [{"kind": "text", "value": "Ready"}]
        });
        let mut registry = TransformTemplateOutputFunctionRegistry::new();
        registry.register(conversion_cem_tree_format_function_descriptor(
            "acme.format-tree",
        ));
        let request = TransformTemplateEncodeBindingRequest::new(
            subject.clone(),
            TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
        )
        .with_subject_type("cem-ast-node")
        .with_options(TransformTemplateEncodeOptions {
            formatter: Some("cem.format-tree".to_owned()),
            formatter_profile: Some("acme.format-tree".to_owned()),
            canonical: true,
            ..TransformTemplateEncodeOptions::default()
        });
        let binding = registry
            .resolve_format_binding(&request, &BTreeSet::new())
            .expect("formatter binding resolves");

        let error = execute_test_conversion_cem_tree_output_stage(stage, &binding, &subject)
            .expect_err("alias formatter without extends is rejected");

        assert!(error.contains(
            "CEMT formatter `acme.format-tree` selected for canonical `cem.format-tree` must extend `cem.format-tree`"
        ));
    }

    fn execute_builtin_csv_formatter_profile_with_options(
        profile: &str,
        options: BTreeMap<String, String>,
    ) -> Result<(Value, ConversionOutputPipelineStageExecution), String> {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let target =
            TransformTemplateEncodingTarget::new(CSV_CONTENT_TYPE, CSV_SCHEMA_URI, "csv-document");
        let stage = cem_tree_cemt_output_stage(
            &environment,
            CemTreeCemtOutputStageSpec {
                adapter_id: "csv-format-cemt",
                artifact_kind: "formatter",
                declaration_element: "{format-function",
                function_kind: TransformTemplateOutputFunctionKind::Format,
                function_name: "csv.format-document",
                role: "formatter",
            },
            &target,
            Some(profile),
            Some("csv.format-document"),
        )?;
        let parse_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: stage.template_uri.clone(),
                    bytes: stage.template_bytes.clone(),
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });
        if !parse_response.diagnostics.is_empty() {
            return Err(parse_response
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; "));
        }
        let formatter = parse_response
            .module_options
            .output_functions
            .iter()
            .find(|function| {
                function.kind == TransformTemplateOutputFunctionKind::Format
                    && function.name == "csv.format-document"
            })
            .cloned()
            .ok_or_else(|| {
                "CSV formatter asset did not declare `csv.format-document`".to_owned()
            })?;
        let mut function_registry = TransformTemplateOutputFunctionRegistry::new();
        function_registry.register(formatter);
        let subject = serde_json::json!({
            "kind": "csv-table",
            "rows": [
                {
                    "index": 0,
                    "fieldCount": 4,
                    "byteOffset": 0,
                    "byteLength": 21,
                    "recordEndingSourceMap": csv_test_source_map(20, 1),
                    "sourceRange": {
                        "byteOffset": 0,
                        "byteLength": 21,
                        "line": 1,
                        "column": 1,
                        "endLine": 2,
                        "endColumn": 1
                    },
                    "fields": [
                        {
                            "index": 0,
                            "value": "id",
                            "quoted": false,
                            "byteOffset": 0,
                            "byteLength": 2,
                            "sourceMap": csv_test_source_map(0, 2),
                            "sourceRange": {
                                "byteOffset": 0,
                                "byteLength": 2,
                                "line": 1,
                                "column": 1,
                                "endLine": 1,
                                "endColumn": 3
                            }
                        },
                        {
                            "index": 1,
                            "value": "name",
                            "delimiterBeforeSourceMap": csv_test_source_map(2, 1),
                            "sourceMap": csv_test_source_map(3, 4)
                        },
                        {"index": 2, "value": "score"},
                        {"index": 3, "value": "amount"}
                    ]
                },
                {
                    "index": 1,
                    "fields": [
                        {"index": 0, "value": "1"},
                        {"index": 1, "value": "Ada"},
                        {"index": 2, "value": "7"},
                        {"index": 3, "value": "12.30"}
                    ]
                },
                {
                    "index": 2,
                    "fields": [
                        {"index": 0, "value": "20"},
                        {"index": 1, "value": "Lin"},
                        {"index": 2, "value": "120"},
                        {"index": 3, "value": "3.5"}
                    ]
                }
            ]
        });
        let request = TransformTemplateEncodeBindingRequest::new(subject.clone(), target)
            .with_subject_type("csv-document")
            .with_options(TransformTemplateEncodeOptions {
                formatter: Some("csv.format-document".to_owned()),
                formatter_profile: Some(profile.to_owned()),
                formatter_options: options,
                canonical: profile == "compact",
                line_ending: Some("lf".to_owned()),
                ..TransformTemplateEncodeOptions::default()
            });
        let binding = function_registry
            .resolve_format_binding(&request, &BTreeSet::new())
            .map_err(|error| error.diagnostic(None).message)?;

        execute_conversion_cem_tree_output_stage(&environment, stage, &binding, &subject)
    }

    fn execute_builtin_csv_formatter_profile(
        profile: &str,
    ) -> Result<(Value, ConversionOutputPipelineStageExecution), String> {
        execute_builtin_csv_formatter_profile_with_options(profile, BTreeMap::new())
    }

    #[test]
    fn cem_ml_and_csv_output_cemt_assets_do_not_use_json_ast_boundaries() {
        for (path, source) in [
            (
                "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt",
                include_str!("../schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt"),
            ),
            (
                "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt",
                include_str!("../schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt"),
            ),
            (
                "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt",
                include_str!(
                    "../schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt"
                ),
            ),
            (
                "schema-packages/cem-ml/v1/formatters/cem-tree-helpers.cemt",
                include_str!("../schema-packages/cem-ml/v1/formatters/cem-tree-helpers.cemt"),
            ),
            (
                "schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt",
                include_str!(
                    "../schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt"
                ),
            ),
            (
                "schema-packages/csv/v1/formatters/compact.cemt",
                include_str!("../schema-packages/csv/v1/formatters/compact.cemt"),
            ),
            (
                "schema-packages/csv/v1/formatters/pretty.cemt",
                include_str!("../schema-packages/csv/v1/formatters/pretty.cemt"),
            ),
            (
                "schema-packages/csv/v1/formatters/tabular.cemt",
                include_str!("../schema-packages/csv/v1/formatters/tabular.cemt"),
            ),
            (
                "schema-packages/json/v1/formatters/compact.cemt",
                include_str!("../schema-packages/json/v1/formatters/compact.cemt"),
            ),
            (
                "schema-packages/json/v1/formatters/pretty.cemt",
                include_str!("../schema-packages/json/v1/formatters/pretty.cemt"),
            ),
            (
                "schema-packages/json/v1/formatters/tabular.cemt",
                include_str!("../schema-packages/json/v1/formatters/tabular.cemt"),
            ),
            (
                "schema-packages/json/v1/formatters/json-format-document.cemt",
                include_str!("../schema-packages/json/v1/formatters/json-format-document.cemt"),
            ),
            (
                "schema-packages/json/v1/colorizers/terminal.cemt",
                include_str!("../schema-packages/json/v1/colorizers/terminal.cemt"),
            ),
            (
                "schema-packages/json/v1/colorizers/html.cemt",
                include_str!("../schema-packages/json/v1/colorizers/html.cemt"),
            ),
            (
                "schema-packages/json/v1/colorizers/md.cemt",
                include_str!("../schema-packages/json/v1/colorizers/md.cemt"),
            ),
            (
                "schema-packages/json/v1/colorizers/json-color-document.cemt",
                include_str!("../schema-packages/json/v1/colorizers/json-color-document.cemt"),
            ),
            (
                "schema-packages/json-schema/v1/formatters/json-schema-format-document.cemt",
                include_str!(
                    "../schema-packages/json-schema/v1/formatters/json-schema-format-document.cemt"
                ),
            ),
            (
                "schema-packages/json-schema/v1/colorizers/json-schema-color-document.cemt",
                include_str!(
                    "../schema-packages/json-schema/v1/colorizers/json-schema-color-document.cemt"
                ),
            ),
        ] {
            assert!(
                !source.contains("@subject=\"json\""),
                "{path} must not declare internal AST subject as JSON"
            );
            assert!(
                !source.contains("@subject=\"tokens\""),
                "{path} must not declare internal AST subject as token streams"
            );
            assert!(
                !source.contains("@type=\"json\""),
                "{path} must not type internal AST parameters as JSON"
            );
            assert!(
                !source.contains("@returns=\"json\""),
                "{path} must not type internal AST returns as JSON"
            );
            assert!(
                !source.contains("@produces=\"tokens\""),
                "{path} must not produce token-stream boundaries for internal AST output"
            );
        }
    }

    fn csv_test_output_span(start: u64, len: u32) -> Value {
        serde_json::json!({
            "outputRange": {
                "start": start,
                "len": len
            },
            "origin": {
                "frames": [{
                    "source_id": 0,
                    "span": {
                        "kind": "Single",
                        "ranges": {
                            "start": start,
                            "len": len
                        }
                    },
                    "transform": {
                        "kind": "ContentTypeTransform",
                        "content_type": CSV_CONTENT_TYPE
                    }
                }]
            }
        })
    }

    fn csv_test_source_map(start: u64, len: u32) -> Value {
        csv_test_output_span(start, len)["origin"].clone()
    }

    fn yaml_test_output_span(start: u64, len: u32) -> Value {
        serde_json::json!({
            "outputRange": {
                "start": start,
                "len": len
            },
            "origin": {
                "frames": [{
                    "source_id": 1,
                    "span": {
                        "kind": "Single",
                        "ranges": {
                            "start": start,
                            "len": len
                        }
                    },
                    "transform": {
                        "kind": "ContentTypeTransform",
                        "content_type": YAML_CONTENT_TYPE
                    }
                }]
            }
        })
    }

    fn yaml_test_source_map(start: u64, len: u32) -> Value {
        yaml_test_output_span(start, len)["origin"].clone()
    }

    fn json_test_output_span(start: u64, len: u32) -> Value {
        serde_json::json!({
            "outputRange": {
                "start": start,
                "len": len
            },
            "origin": {
                "frames": [{
                    "source_id": 1,
                    "span": {
                        "kind": "Single",
                        "ranges": {
                            "start": start,
                            "len": len
                        }
                    },
                    "transform": {
                        "kind": "ContentTypeTransform",
                        "content_type": JSON_CONTENT_TYPE
                    }
                }]
            }
        })
    }

    fn json_test_source_map(start: u64, len: u32) -> Value {
        json_test_output_span(start, len)["origin"].clone()
    }

    fn markdown_test_output_span(start: u64, len: u32) -> Value {
        serde_json::json!({
            "outputRange": {
                "start": start,
                "len": len
            },
            "origin": {
                "frames": [{
                    "source_id": 1,
                    "span": {
                        "kind": "Single",
                        "ranges": {
                            "start": start,
                            "len": len
                        }
                    },
                    "transform": {
                        "kind": "ContentTypeTransform",
                        "content_type": MARKDOWN_CONTENT_TYPE
                    }
                }]
            }
        })
    }

    fn markdown_test_source_map(start: u64, len: u32) -> Value {
        markdown_test_output_span(start, len)["origin"].clone()
    }

    #[test]
    fn builtin_yaml_lifecycle_output_pipeline_formats_typed_ast_without_json_bridge() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "yaml-stream",
            "lineEnding": "lf",
            "documents": [{
                "index": 0,
                "root": {
                    "kind": "mapping",
                    "mapping": [
                        {
                            "index": 0,
                            "key": {
                                "kind": "scalar",
                                "value": "name",
                                "style": "plain",
                                "implicitKind": "string",
                                "sourceMap": yaml_test_source_map(0, 4)
                            },
                            "value": {
                                "kind": "scalar",
                                "value": "Ada",
                                "style": "plain",
                                "implicitKind": "string",
                                "sourceMap": yaml_test_source_map(6, 3)
                            }
                        },
                        {
                            "index": 1,
                            "key": {
                                "kind": "scalar",
                                "value": "active",
                                "style": "plain",
                                "implicitKind": "string",
                                "sourceMap": yaml_test_source_map(10, 6)
                            },
                            "value": {
                                "kind": "scalar",
                                "value": "true",
                                "style": "plain",
                                "implicitKind": "boolean",
                                "sourceMap": yaml_test_source_map(18, 4)
                            }
                        }
                    ]
                }
            }]
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            ..ScopeConfig::default()
        };

        let execution = execute_yaml_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:yaml-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("name: Ada\nactive: true\n")
        );
        assert!(
            matches!(
                execution.format_execution,
                Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                    ref adapter_id,
                    ref function_name,
                    ref body_function_name,
                    ..
                }) if adapter_id == "yaml-format-cemt"
                    && function_name == "yaml.format-document"
                    && body_function_name.as_deref() == Some("yaml.format-document")
            ),
            "{:?}",
            execution.format_execution
        );
        assert_eq!(execution.color_execution, None);
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted YAML CEM tree");
        assert_eq!(formatted.value["kind"], "cem-tree");
        assert_eq!(formatted.value["contentType"], YAML_CONTENT_TYPE);
        assert_eq!(formatted.value["category"], "yaml-document");
        assert_eq!(formatted.value["formatterProfile"], "tabular");
        assert_eq!(execution.output_spans.len(), 4);
        let first_span = &execution.output_spans[0];
        assert_eq!(first_span.output_range.start, 0);
        assert_eq!(first_span.output_range.len, 4);
        let crate::source_map::FrameSpan::Single(first_origin) = first_span.origin.frames[0].span
        else {
            panic!("YAML output span should retain a single origin range");
        };
        assert_eq!(first_origin.start, 0);
        assert_eq!(first_origin.len, 4);
    }

    #[test]
    fn builtin_yaml_lifecycle_output_pipeline_renders_stream_directives_from_package_cemt() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "yaml-stream",
            "lineEnding": "lf",
            "directives": [
                {
                    "index": 0,
                    "name": "YAML",
                    "value": "1.2",
                    "sourceMap": yaml_test_source_map(0, 9)
                },
                {
                    "index": 1,
                    "name": "TAG",
                    "value": "!e! tag:example.com,2026:",
                    "sourceMap": yaml_test_source_map(10, 34)
                }
            ],
            "documents": [{
                "index": 0,
                "sourceMap": yaml_test_source_map(45, 3),
                "root": {
                    "kind": "mapping",
                    "mapping": [{
                        "index": 0,
                        "key": {
                            "kind": "scalar",
                            "value": "name",
                            "style": "plain",
                            "implicitKind": "string"
                        },
                        "value": {
                            "kind": "scalar",
                            "value": "Ada",
                            "style": "plain",
                            "implicitKind": "string"
                        }
                    }]
                }
            }]
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            ..ScopeConfig::default()
        };

        let execution = execute_yaml_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:yaml-directive-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("%YAML 1.2\n%TAG !e! tag:example.com,2026:\n---\nname: Ada\n")
        );
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted YAML CEM tree");
        assert_eq!(formatted.value["nodes"][2]["kind"], "yaml.directive-marker");
        assert_eq!(formatted.value["nodes"][3]["kind"], "yaml.directive-name");
        assert_eq!(formatted.value["nodes"][5]["kind"], "yaml.directive-value");
        assert_eq!(formatted.value["nodes"][8]["kind"], "yaml.directive-name");
        assert_eq!(formatted.value["nodes"][12]["kind"], "yaml.document-start");
    }

    #[test]
    fn builtin_yaml_lifecycle_output_pipeline_renders_stream_comments_from_package_cemt() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "yaml-stream",
            "lineEnding": "lf",
            "comments": [
                {
                    "index": 0,
                    "text": "# header",
                    "value": "header",
                    "indent": "",
                    "placement": "line",
                    "sourceMap": yaml_test_source_map(0, 8)
                },
                {
                    "index": 1,
                    "text": "# inline comment",
                    "value": "inline comment",
                    "indent": "",
                    "placement": "inline",
                    "sourceMap": yaml_test_source_map(20, 16)
                },
                {
                    "index": 2,
                    "text": "# tail",
                    "value": "tail",
                    "indent": "  ",
                    "placement": "line",
                    "sourceMap": yaml_test_source_map(40, 6)
                }
            ],
            "documents": [{
                "index": 0,
                "root": {
                    "kind": "mapping",
                    "mapping": [{
                        "index": 0,
                        "key": {
                            "kind": "scalar",
                            "value": "name",
                            "style": "plain",
                            "implicitKind": "string"
                        },
                        "value": {
                            "kind": "scalar",
                            "value": "Ada",
                            "style": "plain",
                            "implicitKind": "string"
                        }
                    }]
                }
            }]
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            ..ScopeConfig::default()
        };

        let execution = execute_yaml_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:yaml-comment-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("# header\n# inline comment\n  # tail\nname: Ada\n")
        );
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted YAML CEM tree");
        assert_eq!(formatted.value["nodes"][2]["kind"], "yaml.comment");
        assert_eq!(formatted.value["nodes"][2]["role"], "syntax.comment");
        assert_eq!(formatted.value["nodes"][6]["kind"], "yaml.indent");
        assert_eq!(formatted.value["nodes"][7]["kind"], "yaml.comment");
    }

    #[test]
    fn builtin_yaml_lifecycle_output_pipeline_interleaves_comments_by_source_position() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let source = b"# header\nname: Ada # inline name\ndetails:\n  # nested\n  role: admin # inline role\n  active: true\n# tail\n";
        let (document, projection_diagnostics) =
            crate::validation::yaml::yaml_stream_value_from_source_bytes(
                crate::validation::yaml::YamlSourceValidationRequest {
                    bytes: source,
                    source_uri: "memory://yaml-comment-order.yaml",
                    content_type: Some(YAML_CONTENT_TYPE),
                },
            );
        assert!(
            projection_diagnostics.is_empty(),
            "{projection_diagnostics:?}"
        );
        let document = document.expect("valid YAML projects a typed stream");
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            ..ScopeConfig::default()
        };

        let execution = execute_yaml_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:yaml-comment-source-order-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some(
                "# header\nname: Ada # inline name\ndetails:\n    # nested\n    role: admin # inline role\n    active: true\n# tail\n"
            )
        );
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted YAML CEM tree");
        let comment_texts = formatted.value["nodes"]
            .as_array()
            .expect("format nodes")
            .iter()
            .filter(|node| node["kind"] == "yaml.comment")
            .map(|node| node["text"].as_str().unwrap_or("").to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            comment_texts,
            [
                "# header",
                "# inline name",
                "# nested",
                "# inline role",
                "# tail"
            ]
        );
    }

    #[test]
    fn builtin_yaml_lifecycle_output_pipeline_wraps_html_color_pre() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "yaml-stream",
            "documents": [{
                "index": 0,
                "root": {
                    "kind": "mapping",
                    "mapping": [{
                        "index": 0,
                        "key": {
                            "kind": "scalar",
                            "value": "name",
                            "style": "plain",
                            "implicitKind": "string"
                        },
                        "value": {
                            "kind": "scalar",
                            "value": "Ada",
                            "style": "plain",
                            "implicitKind": "string"
                        }
                    }]
                }
            }]
        });
        let target_scope = ScopeConfig {
            cemt_color_profile: Some("html".to_owned()),
            cemt_formatter_options: BTreeMap::from([("tabSize".to_owned(), "6".to_owned())]),
            ..ScopeConfig::default()
        };

        let execution = execute_yaml_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:yaml-html-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        let output = execution.output.as_ref().and_then(Value::as_str).unwrap();
        assert!(output.starts_with(&yaml_html_preview_prefix(6)), "{output}");
        assert!(output.ends_with("</pre>\n"), "{output}");
        assert!(output.contains(r#"data-role="syntax.name""#), "{output}");
        assert_eq!(
            html_text_content(output.strip_suffix('\n').unwrap_or(output)),
            "name: Ada\n"
        );
        assert!(
            matches!(
                execution.color_execution,
                Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                    ref adapter_id,
                    ref function_name,
                    ref body_function_name,
                    ..
                }) if adapter_id == "yaml-color-cemt"
                    && function_name == "yaml.color-document"
                    && body_function_name.as_deref() == Some("yaml.color-document")
            ),
            "{:?}",
            execution.color_execution
        );
    }

    #[test]
    fn builtin_yaml_formatter_profiles_execute_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "yaml-stream",
            "lineEnding": "lf",
            "documents": [{
                "index": 0,
                "root": {
                    "kind": "mapping",
                    "mapping": [
                        {
                            "index": 0,
                            "key": {
                                "kind": "scalar",
                                "value": "name",
                                "style": "plain",
                                "implicitKind": "string"
                            },
                            "value": {
                                "kind": "scalar",
                                "value": "Ada",
                                "style": "plain",
                                "implicitKind": "string"
                            }
                        },
                        {
                            "index": 1,
                            "key": {
                                "kind": "scalar",
                                "value": "code",
                                "style": "plain",
                                "implicitKind": "string"
                            },
                            "value": {
                                "kind": "scalar",
                                "value": "123",
                                "style": "plain",
                                "implicitKind": "string"
                            }
                        },
                        {
                            "index": 2,
                            "key": {
                                "kind": "scalar",
                                "value": "items",
                                "style": "plain",
                                "implicitKind": "string"
                            },
                            "value": {
                                "kind": "sequence",
                                "sequence": [
                                    {
                                        "kind": "scalar",
                                        "value": "one",
                                        "style": "plain",
                                        "implicitKind": "string"
                                    }
                                ]
                            }
                        }
                    ]
                }
            }]
        });

        for (profile, tree_profile, layout) in [
            ("compact", "compact", "compact-block-document"),
            ("pretty", "yaml.pretty", "pretty-block-document"),
            ("tabular", "tabular", "tabular-block-document"),
        ] {
            let target_scope = ScopeConfig {
                cemt_formatter_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_yaml_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:yaml-profile-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.format_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "yaml-format-cemt"
                        && function_name == "yaml.format-document"
                        && body_function_name.as_deref() == Some("yaml.format-document")
                ),
                "{profile}: {:?}",
                execution.format_execution
            );
            assert_eq!(
                execution.output.as_ref().and_then(Value::as_str),
                Some("name: Ada\ncode: \"123\"\nitems:\n    - one\n"),
                "{profile}"
            );
            let formatted = execution
                .formatted_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} formatted tree"));
            assert_eq!(formatted.value["formatterProfile"], tree_profile);
            assert_eq!(formatted.value["formatNodes"][1]["value"]["layout"], layout);
        }
    }

    #[test]
    fn builtin_yaml_colorizer_profiles_execute_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "yaml-stream",
            "lineEnding": "lf",
            "documents": [{
                "index": 0,
                "root": {
                    "kind": "mapping",
                    "mapping": [{
                        "index": 0,
                        "key": {
                            "kind": "scalar",
                            "value": "name",
                            "style": "plain",
                            "implicitKind": "string"
                        },
                        "value": {
                            "kind": "scalar",
                            "value": "Ada",
                            "style": "plain",
                            "implicitKind": "string"
                        }
                    }]
                }
            }]
        });

        for (profile, output, style_key, style_value) in [
            ("terminal", "terminal", "terminalCapability", "auto"),
            ("html", "html", "htmlMode", "classes"),
            ("md", "md", "wrapper", "span"),
        ] {
            let target_scope = ScopeConfig {
                cemt_color_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_yaml_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:yaml-color-profile-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.color_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "yaml-color-cemt"
                        && function_name == "yaml.color-document"
                        && body_function_name.as_deref() == Some("yaml.color-document")
                ),
                "{profile}: {:?}",
                execution.color_execution
            );
            let colored = execution
                .colored_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} colored tree"));
            assert_eq!(colored.value["colorProfile"], profile);
            assert_eq!(colored.value["colorOutput"], output);
            assert_eq!(colored.value["nodes"][2]["style"][style_key], style_value);
            assert_eq!(
                colored.value["nodes"][2]["value"]["colorRole"],
                "syntax.name"
            );
            if profile == "html" {
                let output = execution.output.as_ref().and_then(Value::as_str).unwrap();
                assert!(output.starts_with(&yaml_html_preview_prefix(default_formatter_tab_size())));
                assert!(output.contains(r#"data-role="syntax.name""#), "{output}");
            } else {
                assert_eq!(
                    strip_ansi_codes(execution.output.as_ref().and_then(Value::as_str).unwrap()),
                    "name: Ada\n"
                );
            }
        }
    }

    #[test]
    fn builtin_json_lifecycle_output_pipeline_formats_typed_ast_without_value_bridge() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-document",
            "lineEnding": "lf",
            "root": {
                "kind": "object",
                "sourceMap": json_test_source_map(0, 28),
                "members": [
                    {
                        "index": 0,
                        "name": "name",
                        "nameLexeme": "\"name\"",
                        "nameSourceMap": json_test_source_map(1, 6),
                        "sourceMap": json_test_source_map(1, 12),
                        "value": {
                            "kind": "string",
                            "value": "Ada",
                            "lexeme": "\"Ada\"",
                            "sourceMap": json_test_source_map(8, 5)
                        }
                    },
                    {
                        "index": 1,
                        "name": "active",
                        "nameLexeme": "\"active\"",
                        "nameSourceMap": json_test_source_map(15, 8),
                        "sourceMap": json_test_source_map(15, 12),
                        "value": {
                            "kind": "boolean",
                            "value": true,
                            "sourceMap": json_test_source_map(24, 4)
                        }
                    }
                ]
            }
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            ..ScopeConfig::default()
        };

        let execution = execute_json_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:json-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("{   \"name\": \"Ada\"\n,   \"active\": true\n}\n")
        );
        assert!(
            matches!(
                execution.format_execution,
                Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                    ref adapter_id,
                    ref function_name,
                    ref body_function_name,
                    ..
                }) if adapter_id == "json-format-cemt"
                    && function_name == "json.format-document"
                    && body_function_name.as_deref() == Some("json.format-document")
            ),
            "{:?}",
            execution.format_execution
        );
        assert_eq!(execution.color_execution, None);
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted JSON CEM tree");
        assert_eq!(formatted.value["kind"], "cem-tree");
        assert_eq!(formatted.value["contentType"], JSON_CONTENT_TYPE);
        assert_eq!(formatted.value["category"], "json-document");
        assert_eq!(formatted.value["formatterProfile"], "tabular");
        let expected_name_origin = json_test_source_map(1, 6);
        let has_formatted_name_span = formatted.value["nodes"]
            .as_array()
            .expect("formatted JSON CEM tree nodes")
            .iter()
            .any(|node| {
                node.get("outputSpan").and_then(|span| span.get("origin"))
                    == Some(&expected_name_origin)
            });
        assert!(
            has_formatted_name_span,
            "formatted JSON tree should retain member-name source map spans"
        );
        let has_name_span = execution.output_spans.iter().any(|span| {
            let crate::source_map::FrameSpan::Single(origin) = span.origin.frames[0].span else {
                return false;
            };
            origin.start == 1 && origin.len == 6
        });
        assert!(
            has_name_span,
            "JSON writer should retain member-name source map spans"
        );
    }

    #[test]
    fn builtin_json_lifecycle_output_pipeline_preserves_duplicate_members() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-document",
            "root": {
                "kind": "object",
                "members": [
                    {
                        "index": 0,
                        "name": "name",
                        "nameLexeme": "\"name\"",
                        "value": {
                            "kind": "string",
                            "value": "Ada",
                            "lexeme": "\"Ada\""
                        }
                    },
                    {
                        "index": 1,
                        "name": "name",
                        "nameLexeme": "\"name\"",
                        "value": {
                            "kind": "string",
                            "value": "Lin",
                            "lexeme": "\"Lin\""
                        }
                    }
                ]
            }
        });

        let execution = execute_json_document_output_pipeline_with_environment(
            &environment,
            document,
            &ScopeConfig::default(),
            Some("builtin:json-duplicate-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("{\"name\":\"Ada\",\"name\":\"Lin\"}\n")
        );
    }

    #[test]
    fn builtin_json_lifecycle_output_pipeline_wraps_html_color_pre() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-document",
            "root": {
                "kind": "object",
                "members": [{
                    "index": 0,
                    "name": "name",
                    "nameLexeme": "\"name\"",
                    "value": {
                        "kind": "string",
                        "value": "Ada",
                        "lexeme": "\"Ada\""
                    }
                }]
            }
        });
        let target_scope = ScopeConfig {
            cemt_color_profile: Some("html".to_owned()),
            cemt_formatter_options: BTreeMap::from([("tabSize".to_owned(), "6".to_owned())]),
            ..ScopeConfig::default()
        };

        let execution = execute_json_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:json-html-output"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        let output = execution.output.as_ref().and_then(Value::as_str).unwrap();
        assert!(output.starts_with(&json_html_preview_prefix(6)), "{output}");
        assert!(output.ends_with("</pre>\n"), "{output}");
        assert!(output.contains(r#"data-role="syntax.name""#), "{output}");
        assert_eq!(
            html_text_content(output.strip_suffix('\n').unwrap_or(output)),
            r#"{"name":"Ada"}"#
        );
        assert!(
            matches!(
                execution.color_execution,
                Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                    ref adapter_id,
                    ref function_name,
                    ref body_function_name,
                    ..
                }) if adapter_id == "json-color-cemt"
                    && function_name == "json.color-document"
                    && body_function_name.as_deref() == Some("json.color-document")
            ),
            "{:?}",
            execution.color_execution
        );
    }

    #[test]
    fn builtin_json_formatter_profiles_execute_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-document",
            "lineEnding": "lf",
            "root": {
                "kind": "object",
                "members": [
                    {
                        "index": 0,
                        "name": "name",
                        "nameLexeme": "\"name\"",
                        "value": {
                            "kind": "string",
                            "value": "Ada",
                            "lexeme": "\"Ada\""
                        }
                    },
                    {
                        "index": 1,
                        "name": "site",
                        "nameLexeme": "\"site\"",
                        "value": {
                            "kind": "object",
                            "members": [
                                {
                                    "index": 0,
                                    "name": "title",
                                    "nameLexeme": "\"title\"",
                                    "value": {
                                        "kind": "string",
                                        "value": "CEM Demo",
                                        "lexeme": "\"CEM Demo\""
                                    }
                                }
                            ]
                        }
                    },
                    {
                        "index": 2,
                        "name": "items",
                        "nameLexeme": "\"items\"",
                        "value": {
                            "kind": "array",
                            "items": [
                                {
                                    "kind": "number",
                                    "lexeme": "1",
                                    "numberKind": "integer"
                                },
                                {
                                    "kind": "boolean",
                                    "value": true
                                }
                            ]
                        }
                    }
                ]
            }
        });

        for (profile, expected_path, tree_profile, layout, expected_output) in [
            (
                "compact",
                "schema-packages/json/v1/formatters/compact.cemt",
                "compact",
                "compact-json-document",
                "{\"name\":\"Ada\",\"site\":{\"title\":\"CEM Demo\"},\"items\":[1,true]}\n",
            ),
            (
                "pretty",
                "schema-packages/json/v1/formatters/pretty.cemt",
                "json.pretty",
                "pretty-json-document",
                "{   \"name\": \"Ada\"\n,   \"site\": \n    {   \"title\": \"CEM Demo\"\n    }\n,   \"items\": \n    [   1\n    ,   true\n    ]\n}\n",
            ),
            (
                "tabular",
                "schema-packages/json/v1/formatters/tabular.cemt",
                "tabular",
                "tabular-json-document",
                "{   \"name\": \"Ada\"\n,   \"site\": \n    {   \"title\": \"CEM Demo\"\n    }\n,   \"items\": \n    [   1\n    ,   true\n]   }\n",
            ),
        ] {
            let target_scope = ScopeConfig {
                cemt_formatter_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_json_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:json-profile-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.format_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "json-format-cemt"
                        && function_name == "json.format-document"
                        && body_function_name.as_deref() == Some("json.format-document")
                ),
                "{profile}: {:?}",
                execution.format_execution
            );
            assert_eq!(
                execution.output.as_ref().and_then(Value::as_str),
                Some(expected_output),
                "{profile}"
            );
            let formatted = execution
                .formatted_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} formatted tree"));
            assert_eq!(formatted.value["formatterProfile"], tree_profile);
            assert_eq!(formatted.value["formatNodes"][1]["value"]["layout"], layout);
            assert_eq!(
                formatted.value["nodes"][0]["value"]["formatterProfile"],
                tree_profile
            );
            assert!(
                builtin_schema_package_artifact_source("json", expected_path)
                    .is_some_and(|source| source.source.contains("{body |")),
                "{profile} formatter asset must be embedded with an executable body"
            );
        }
    }

    #[test]
    fn builtin_json_formatter_applies_comma_and_scope_opening_options() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-document",
            "lineEnding": "lf",
            "root": {
                "kind": "object",
                "members": [
                    {
                        "index": 0,
                        "name": "name",
                        "nameLexeme": "\"name\"",
                        "value": {
                            "kind": "string",
                            "value": "Ada",
                            "lexeme": "\"Ada\""
                        }
                    },
                    {
                        "index": 1,
                        "name": "items",
                        "nameLexeme": "\"items\"",
                        "value": {
                            "kind": "array",
                            "items": [
                                {
                                    "kind": "number",
                                    "lexeme": "1",
                                    "numberKind": "integer"
                                },
                                {
                                    "kind": "boolean",
                                    "value": true
                                }
                            ]
                        }
                    }
                ]
            }
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("pretty".to_owned()),
            cemt_formatter_options: BTreeMap::from([
                ("leadingComma".to_owned(), "false".to_owned()),
                ("scopeOpeningNewLine".to_owned(), "true".to_owned()),
            ]),
            ..ScopeConfig::default()
        };

        let execution = execute_json_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:json-layout-options"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some(
                "{\n    \"name\": \"Ada\",\n    \"items\": [\n        1,\n        true\n    ]\n}\n"
            )
        );
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted JSON CEM tree");
        assert_eq!(
            formatted.value["formatNodes"][1]["value"]["leadingComma"],
            false
        );
        assert_eq!(
            formatted.value["formatNodes"][1]["value"]["scopeOpeningNewLine"],
            true
        );
    }

    #[test]
    fn builtin_json_colorizer_profiles_execute_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-document",
            "root": {
                "kind": "object",
                "members": [{
                    "index": 0,
                    "name": "name",
                    "nameLexeme": "\"name\"",
                    "value": {
                        "kind": "string",
                        "value": "Ada",
                        "lexeme": "\"Ada\""
                    }
                }]
            }
        });

        for (profile, output, style_key, style_value) in [
            ("terminal", "terminal", "terminalCapability", "auto"),
            ("html", "html", "htmlMode", "classes"),
            ("md", "md", "wrapper", "span"),
        ] {
            let target_scope = ScopeConfig {
                cemt_color_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_json_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:json-color-profile-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.color_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "json-color-cemt"
                        && function_name == "json.color-document"
                        && body_function_name.as_deref() == Some("json.color-document")
                ),
                "{profile}: {:?}",
                execution.color_execution
            );
            let colored = execution
                .colored_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} colored tree"));
            assert_eq!(colored.value["colorProfile"], profile);
            assert_eq!(colored.value["colorOutput"], output);
            assert_eq!(colored.value["nodes"][3]["style"][style_key], style_value);
            assert_eq!(
                colored.value["nodes"][3]["value"]["colorRole"],
                "syntax.name"
            );
            if profile == "html" {
                let output = execution.output.as_ref().and_then(Value::as_str).unwrap();
                assert!(output.starts_with(&json_html_preview_prefix(default_formatter_tab_size())));
                assert!(output.contains(r#"data-role="syntax.name""#), "{output}");
            } else {
                assert_eq!(
                    strip_ansi_codes(execution.output.as_ref().and_then(Value::as_str).unwrap()),
                    "{\"name\":\"Ada\"}\n"
                );
            }
        }
    }

    #[test]
    fn builtin_json_schema_lifecycle_output_pipeline_executes_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-schema-document",
            "contentType": JSON_SCHEMA_CONTENT_TYPE,
            "schema": JSON_SCHEMA_SCHEMA_URI,
            "dialect": "https://json-schema.org/draft/2020-12/schema",
            "json": {
                "kind": "json-document",
                "lineEnding": "lf",
                "root": {
                    "kind": "object",
                    "members": [
                        {
                            "index": 0,
                            "name": "$schema",
                            "nameLexeme": "\"$schema\"",
                            "value": {
                                "kind": "string",
                                "value": "https://json-schema.org/draft/2020-12/schema",
                                "lexeme": "\"https://json-schema.org/draft/2020-12/schema\""
                            }
                        },
                        {
                            "index": 1,
                            "name": "type",
                            "nameLexeme": "\"type\"",
                            "value": {
                                "kind": "string",
                                "value": "object",
                                "lexeme": "\"object\""
                            }
                        },
                        {
                            "index": 2,
                            "name": "properties",
                            "nameLexeme": "\"properties\"",
                            "value": {
                                "kind": "object",
                                "members": [
                                    {
                                        "index": 0,
                                        "name": "title",
                                        "nameLexeme": "\"title\"",
                                        "value": {
                                            "kind": "object",
                                            "members": [
                                                {
                                                    "index": 0,
                                                    "name": "type",
                                                    "nameLexeme": "\"type\"",
                                                    "value": {
                                                        "kind": "string",
                                                        "value": "string",
                                                        "lexeme": "\"string\""
                                                    }
                                                }
                                            ]
                                        }
                                    }
                                ]
                            }
                        }
                    ]
                }
            }
        });

        for (profile, tree_profile, layout, expected_output) in [
            (
                "compact",
                "compact",
                "compact-json-schema-document",
                "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\"}}}\n",
            ),
            (
                "pretty",
                "json.pretty",
                "pretty-json-schema-document",
                "{   \"$schema\": \"https://json-schema.org/draft/2020-12/schema\"\n,   \"type\": \"object\"\n,   \"properties\": \n    {   \"title\": \n        {   \"type\": \"string\"\n        }\n    }\n}\n",
            ),
            (
                "tabular",
                "tabular",
                "tabular-json-schema-document",
                "{   \"$schema\": \"https://json-schema.org/draft/2020-12/schema\"\n,   \"type\": \"object\"\n,   \"properties\": \n    {   \"title\": \n        {   \"type\": \"string\"\n}   }   }\n",
            ),
        ] {
            let target_scope = ScopeConfig {
                cemt_formatter_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_json_schema_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:json-schema-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.format_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "json-schema-format-cemt"
                        && function_name == "json-schema.format-document"
                        && body_function_name.as_deref() == Some("json-schema.format-document")
                ),
                "{profile}: {:?}",
                execution.format_execution
            );
            assert_eq!(
                execution.output.as_ref().and_then(Value::as_str),
                Some(expected_output),
                "{profile}"
            );
            let formatted = execution
                .formatted_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} formatted tree"));
            assert_eq!(formatted.value["kind"], "cem-tree");
            assert_eq!(formatted.value["contentType"], JSON_SCHEMA_CONTENT_TYPE);
            assert_eq!(formatted.value["schema"], JSON_SCHEMA_SCHEMA_URI);
            assert_eq!(formatted.value["category"], "json-schema-document");
            assert_eq!(formatted.value["formatterProfile"], tree_profile);
            assert_eq!(formatted.value["formatNodes"][1]["value"]["layout"], layout);
            assert_eq!(
                formatted.value["nodes"][0]["value"]["name"],
                "json-schema.format-document"
            );
        }

        assert!(
            builtin_schema_package_artifact_source(
                "json-schema",
                "schema-packages/json-schema/v1/formatters/json-schema-format-document.cemt",
            )
            .is_some_and(|source| source.source.contains("{body |")),
            "JSON Schema formatter asset must be embedded with an executable body"
        );
    }

    #[test]
    fn builtin_json_schema_formatter_applies_comma_and_scope_opening_options() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-schema-document",
            "contentType": JSON_SCHEMA_CONTENT_TYPE,
            "schema": JSON_SCHEMA_SCHEMA_URI,
            "dialect": "https://json-schema.org/draft/2020-12/schema",
            "json": {
                "kind": "json-document",
                "lineEnding": "lf",
                "root": {
                    "kind": "object",
                    "members": [
                        {
                            "index": 0,
                            "name": "$schema",
                            "nameLexeme": "\"$schema\"",
                            "value": {
                                "kind": "string",
                                "value": "https://json-schema.org/draft/2020-12/schema",
                                "lexeme": "\"https://json-schema.org/draft/2020-12/schema\""
                            }
                        },
                        {
                            "index": 1,
                            "name": "type",
                            "nameLexeme": "\"type\"",
                            "value": {
                                "kind": "string",
                                "value": "object",
                                "lexeme": "\"object\""
                            }
                        }
                    ]
                }
            }
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("pretty".to_owned()),
            cemt_formatter_options: BTreeMap::from([
                ("leadingComma".to_owned(), "false".to_owned()),
                ("scopeOpeningNewLine".to_owned(), "true".to_owned()),
            ]),
            ..ScopeConfig::default()
        };

        let execution = execute_json_schema_document_output_pipeline_with_environment(
            &environment,
            document,
            &target_scope,
            Some("builtin:json-schema-layout-options"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("{\n    \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n    \"type\": \"object\"\n}\n")
        );
        let formatted = execution
            .formatted_cem_tree
            .as_ref()
            .expect("formatted JSON Schema CEM tree");
        assert_eq!(
            formatted.value["formatNodes"][1]["value"]["leadingComma"],
            false
        );
        assert_eq!(
            formatted.value["formatNodes"][1]["value"]["scopeOpeningNewLine"],
            true
        );
    }

    #[test]
    fn builtin_json_schema_colorizer_profiles_execute_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "json-schema-document",
            "contentType": JSON_SCHEMA_CONTENT_TYPE,
            "schema": JSON_SCHEMA_SCHEMA_URI,
            "dialect": "https://json-schema.org/draft/2020-12/schema",
            "json": {
                "kind": "json-document",
                "root": {
                    "kind": "object",
                    "members": [{
                        "index": 0,
                        "name": "type",
                        "nameLexeme": "\"type\"",
                        "value": {
                            "kind": "string",
                            "value": "object",
                            "lexeme": "\"object\""
                        }
                    }]
                }
            }
        });

        for (profile, output, style_key, style_value) in [
            ("terminal", "terminal", "terminalCapability", "auto"),
            ("html", "html", "htmlMode", "classes"),
            ("md", "md", "wrapper", "span"),
        ] {
            let target_scope = ScopeConfig {
                cemt_color_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_json_schema_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:json-schema-color-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.color_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "json-schema-color-cemt"
                        && function_name == "json-schema.color-document"
                        && body_function_name.as_deref() == Some("json-schema.color-document")
                ),
                "{profile}: {:?}",
                execution.color_execution
            );
            let colored = execution
                .colored_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} colored tree"));
            assert_eq!(colored.value["contentType"], JSON_SCHEMA_CONTENT_TYPE);
            assert_eq!(colored.value["schema"], JSON_SCHEMA_SCHEMA_URI);
            assert_eq!(colored.value["category"], "json-schema-document");
            assert_eq!(colored.value["colorProfile"], profile);
            assert_eq!(colored.value["colorOutput"], output);
            assert_eq!(colored.value["nodes"][3]["style"][style_key], style_value);
            assert_eq!(
                colored.value["nodes"][3]["value"]["colorRole"],
                "syntax.name"
            );
            if profile == "html" {
                let output = execution.output.as_ref().and_then(Value::as_str).unwrap();
                assert!(
                    output.starts_with(&json_schema_html_preview_prefix(
                        default_formatter_tab_size()
                    )),
                    "{output}"
                );
                assert!(output.contains(r#"data-role="syntax.name""#), "{output}");
                assert!(output.ends_with("</pre>\n"), "{output}");
                assert_eq!(
                    html_text_content(output.strip_suffix('\n').unwrap_or(output)),
                    r#"{"type":"object"}"#
                );
            } else {
                assert_eq!(
                    strip_ansi_codes(execution.output.as_ref().and_then(Value::as_str).unwrap()),
                    "{\"type\":\"object\"}\n"
                );
            }
        }

        assert!(
            builtin_schema_package_artifact_source(
                "json-schema",
                "schema-packages/json-schema/v1/colorizers/json-schema-color-document.cemt",
            )
            .is_some_and(|source| source.source.contains("{body |")),
            "JSON Schema colorizer asset must be embedded with an executable body"
        );
    }

    #[test]
    fn builtin_markdown_lifecycle_output_pipeline_executes_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "markdown-document",
            "contentType": MARKDOWN_CONTENT_TYPE,
            "schema": MARKDOWN_SCHEMA_URI,
            "lineEnding": "lf",
            "events": [
                {
                    "index": 0,
                    "kind": "start",
                    "tag": "heading",
                    "level": 1,
                    "sourceMap": markdown_test_source_map(0, 1)
                },
                {
                    "index": 1,
                    "kind": "text",
                    "text": "Release Notes",
                    "sourceMap": markdown_test_source_map(2, 13)
                },
                {
                    "index": 2,
                    "kind": "end",
                    "tag": "heading",
                    "sourceMap": markdown_test_source_map(15, 0)
                },
                {
                    "index": 3,
                    "kind": "start",
                    "tag": "paragraph",
                    "sourceMap": markdown_test_source_map(17, 0)
                },
                {
                    "index": 4,
                    "kind": "text",
                    "text": "Ready",
                    "sourceMap": markdown_test_source_map(17, 5)
                },
                {
                    "index": 5,
                    "kind": "end",
                    "tag": "paragraph",
                    "sourceMap": markdown_test_source_map(22, 0)
                }
            ]
        });

        for (profile, layout, expected_output) in [
            (
                "compact",
                "compact-markdown-events",
                "# Release Notes\n\nReady\n\n",
            ),
            (
                "pretty",
                "pretty-markdown-events",
                "# Release Notes\n\nReady\n\n",
            ),
            (
                "tabular",
                "tabular-markdown-events",
                "# Release Notes\n\nReady\n\n",
            ),
        ] {
            let target_scope = ScopeConfig {
                cemt_formatter_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_markdown_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:markdown-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.format_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "markdown-format-cemt"
                        && function_name == "markdown.format-document"
                        && body_function_name.as_deref() == Some("markdown.format-document")
                ),
                "{profile}: {:?}",
                execution.format_execution
            );
            assert_eq!(
                execution.output.as_ref().and_then(Value::as_str),
                Some(expected_output),
                "{profile}"
            );
            let formatted = execution
                .formatted_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} formatted tree"));
            assert_eq!(formatted.value["kind"], "cem-tree");
            assert_eq!(formatted.value["contentType"], MARKDOWN_CONTENT_TYPE);
            assert_eq!(formatted.value["schema"], MARKDOWN_SCHEMA_URI);
            assert_eq!(formatted.value["category"], "markdown-document");
            assert_eq!(formatted.value["formatterProfile"], profile);
            assert_eq!(formatted.value["formatNodes"][1]["value"]["layout"], layout);
            assert_eq!(
                formatted.value["nodes"][0]["value"]["name"],
                "markdown.format-document"
            );
        }

        assert!(
            builtin_schema_package_artifact_source(
                "markdown",
                "schema-packages/markdown/v1/formatters/markdown-format-document.cemt",
            )
            .is_some_and(|source| source.source.contains("{body |")),
            "Markdown formatter asset must be embedded with an executable body"
        );
    }

    #[test]
    fn builtin_markdown_colorizer_profiles_execute_package_cemt_assets() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let document = serde_json::json!({
            "kind": "markdown-document",
            "contentType": MARKDOWN_CONTENT_TYPE,
            "schema": MARKDOWN_SCHEMA_URI,
            "lineEnding": "lf",
            "events": [
                {
                    "index": 0,
                    "kind": "start",
                    "tag": "heading",
                    "level": 1
                },
                {
                    "index": 1,
                    "kind": "text",
                    "text": "Release Notes"
                },
                {
                    "index": 2,
                    "kind": "end",
                    "tag": "heading"
                }
            ]
        });

        for (profile, output, style_key, style_value) in [
            ("terminal", "terminal", "terminalCapability", "auto"),
            ("html", "html", "htmlMode", "classes"),
            ("md", "md", "wrapper", "span"),
        ] {
            let target_scope = ScopeConfig {
                cemt_color_profile: Some(profile.to_owned()),
                ..ScopeConfig::default()
            };
            let execution = execute_markdown_document_output_pipeline_with_environment(
                &environment,
                document.clone(),
                &target_scope,
                Some("builtin:markdown-color-output"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{profile}: {:?}",
                execution.diagnostics
            );
            assert!(
                matches!(
                    execution.color_execution,
                    Some(ConversionOutputPipelineStageExecution::CemtAdapter {
                        ref adapter_id,
                        ref function_name,
                        ref body_function_name,
                        ..
                    }) if adapter_id == "markdown-color-cemt"
                        && function_name == "markdown.color-document"
                        && body_function_name.as_deref() == Some("markdown.color-document")
                ),
                "{profile}: {:?}",
                execution.color_execution
            );
            let colored = execution
                .colored_cem_tree
                .as_ref()
                .unwrap_or_else(|| panic!("{profile} colored tree"));
            assert_eq!(colored.value["contentType"], MARKDOWN_CONTENT_TYPE);
            assert_eq!(colored.value["schema"], MARKDOWN_SCHEMA_URI);
            assert_eq!(colored.value["category"], "markdown-document");
            assert_eq!(colored.value["colorProfile"], profile);
            assert_eq!(colored.value["colorOutput"], output);
            assert_eq!(colored.value["nodes"][2]["style"][style_key], style_value);
            assert_eq!(
                colored.value["nodes"][2]["value"]["colorRole"],
                "syntax.punctuation"
            );
            if profile == "html" {
                let output = execution.output.as_ref().and_then(Value::as_str).unwrap();
                assert!(
                    output.starts_with(&markdown_html_preview_prefix(default_formatter_tab_size())),
                    "{output}"
                );
                assert!(
                    output.contains(r#"data-role="syntax.punctuation""#),
                    "{output}"
                );
                assert!(output.ends_with("</pre>\n"), "{output}");
                assert_eq!(
                    html_text_content(output.strip_suffix('\n').unwrap_or(output)),
                    "# Release Notes\n\n"
                );
            } else {
                assert_eq!(
                    strip_ansi_codes(execution.output.as_ref().and_then(Value::as_str).unwrap()),
                    "# Release Notes\n\n"
                );
            }
        }

        assert!(
            builtin_schema_package_artifact_source(
                "markdown",
                "schema-packages/markdown/v1/colorizers/markdown-color-document.cemt",
            )
            .is_some_and(|source| source.source.contains("{body |")),
            "Markdown colorizer asset must be embedded with an executable body"
        );
    }

    fn execute_builtin_csv_colorizer_profile(
        profile: &str,
    ) -> Result<
        (
            Value,
            ConversionOutputPipelineStageExecution,
            TransformTemplateEncodeBinding,
        ),
        String,
    > {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let target =
            TransformTemplateEncodingTarget::new(CSV_CONTENT_TYPE, CSV_SCHEMA_URI, "csv-document");
        let stage = cem_tree_cemt_output_stage(
            &environment,
            CemTreeCemtOutputStageSpec {
                adapter_id: "csv-color-cemt",
                artifact_kind: "colorizer",
                declaration_element: "{color-function",
                function_kind: TransformTemplateOutputFunctionKind::Color,
                function_name: "csv.color-document",
                role: "colorizer",
            },
            &target,
            Some(profile),
            Some("csv.color-document"),
        )?;
        let parse_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: stage.template_uri.clone(),
                    bytes: stage.template_bytes.clone(),
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
            });
        if !parse_response.diagnostics.is_empty() {
            return Err(parse_response
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; "));
        }
        let colorizer = parse_response
            .module_options
            .output_functions
            .iter()
            .find(|function| {
                function.kind == TransformTemplateOutputFunctionKind::Color
                    && function.name == "csv.color-document"
            })
            .cloned()
            .ok_or_else(|| "CSV colorizer asset did not declare `csv.color-document`".to_owned())?;
        let mut function_registry = TransformTemplateOutputFunctionRegistry::new();
        function_registry.register(colorizer);
        let subject = serde_json::json!({
            "kind": "cem-tree",
            "contentType": CSV_CONTENT_TYPE,
            "schema": CSV_SCHEMA_URI,
            "category": "csv-document",
            "formatterProfile": "compact",
            "formatNodes": [
                {
                    "kind": "format-marker",
                    "name": "csv.format-document",
                    "formatterRole": "formatter.boundary",
                    "formatterProfile": "compact"
                },
                {
                    "kind": "format-decision",
                    "name": "csv.layout",
                    "formatterRole": "formatter.layout",
                    "formatterProfile": "compact",
                    "value": {
                        "layout": "compact-records",
                        "delimiter": ",",
                        "lineEnding": "lf"
                    }
                }
            ],
            "nodes": [
                {
                    "kind": "csv.field",
                    "writerKind": "token",
                    "text": "id",
                    "role": "syntax.string",
                    "value": {
                        "rowIndex": 0,
                        "fieldIndex": 0,
                        "formatterProfile": "compact"
                    },
                    "outputSpan": csv_test_output_span(0, 2)
                },
                {
                    "kind": "csv.delimiter",
                    "writerKind": "token",
                    "text": ",",
                    "role": "syntax.punctuation",
                    "value": {
                        "rowIndex": 0,
                        "fieldIndex": 1,
                        "formatterProfile": "compact"
                    },
                    "outputSpan": csv_test_output_span(2, 1)
                },
                {
                    "kind": "csv.field",
                    "writerKind": "token",
                    "text": "name",
                    "role": "syntax.string",
                    "value": {
                        "rowIndex": 0,
                        "fieldIndex": 1,
                        "formatterProfile": "compact"
                    },
                    "outputSpan": csv_test_output_span(3, 4)
                },
                {
                    "kind": "csv.record-ending",
                    "writerKind": "token",
                    "text": "\n",
                    "role": "syntax.punctuation",
                    "value": {
                        "rowIndex": 0,
                        "formatterProfile": "compact"
                    },
                    "outputSpan": csv_test_output_span(7, 1)
                }
            ]
        });
        let request = TransformTemplateEncodeBindingRequest::new(subject.clone(), target)
            .with_subject_type("cem-tree")
            .with_options(TransformTemplateEncodeOptions {
                colorizer: Some("csv.color-document".to_owned()),
                color_profile: Some(profile.to_owned()),
                formatter_profile: Some("compact".to_owned()),
                canonical: false,
                ..TransformTemplateEncodeOptions::default()
            });
        let binding = function_registry
            .resolve_color_binding(&request, &BTreeSet::new())
            .map_err(|error| error.diagnostic(None).message)?
            .into_encode_binding();
        let (colored, execution) =
            execute_conversion_cem_tree_output_stage(&environment, stage, &binding, &subject)?;

        Ok((colored, execution, binding))
    }

    fn writer_node_text(value: &Value) -> String {
        value
            .get("nodes")
            .and_then(Value::as_array)
            .expect("writer node tree")
            .iter()
            .map(|node| node.get("text").and_then(Value::as_str).unwrap_or(""))
            .collect::<String>()
    }

    fn strip_ansi_codes(input: &str) -> String {
        let mut output = String::new();
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' && chars.peek() == Some(&'[') {
                chars.next();
                for code_ch in chars.by_ref() {
                    if code_ch.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            output.push(ch);
        }
        output
    }

    fn html_text_content(input: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        for ch in input.chars() {
            match ch {
                '<' => in_tag = true,
                '>' if in_tag => in_tag = false,
                _ if !in_tag => text.push(ch),
                _ => {}
            }
        }
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
    }

    #[test]
    fn builtin_csv_formatter_profiles_execute_package_cemt_assets() {
        for (profile, expected_path, expected_layout, expected_output) in [
            (
                "compact",
                "schema-packages/csv/v1/formatters/compact.cemt",
                "compact-records",
                "id,name,score,amount\n1,Ada,7,12.30\n20,Lin,120,3.5\n",
            ),
            (
                "pretty",
                "schema-packages/csv/v1/formatters/pretty.cemt",
                "pretty-records",
                "id\t,name\t,score\t,amount\n 1\t,Ada \t,    7\t,12.30 \n20\t,Lin \t,  120\t, 3.5  \n",
            ),
            (
                "tabular",
                "schema-packages/csv/v1/formatters/tabular.cemt",
                "tabular-records",
                "id,name,score,amount\n 1,Ada ,    7,12.30 \n20,Lin ,  120, 3.5  \n",
            ),
        ] {
            let (formatted, execution) = execute_builtin_csv_formatter_profile(profile)
                .unwrap_or_else(|error| panic!("{profile} CSV formatter failed: {error}"));

            assert_eq!(
                execution,
                ConversionOutputPipelineStageExecution::CemtAdapter {
                    adapter_id: "csv-format-cemt".to_owned(),
                    function_name: "csv.format-document".to_owned(),
                    body_function_name: Some("csv.format-document".to_owned()),
                    fallback_function_name: None,
                },
                "{profile}"
            );
            assert_eq!(formatted["kind"], "cem-tree", "{profile}");
            assert_eq!(formatted["contentType"], CSV_CONTENT_TYPE, "{profile}");
            assert_eq!(formatted["schema"], CSV_SCHEMA_URI, "{profile}");
            assert_eq!(formatted["category"], "csv-document", "{profile}");
            assert_eq!(formatted["formatterProfile"], profile, "{profile}");
            assert_eq!(
                formatted["formatNodes"][0]["name"], "csv.format-document",
                "{profile}"
            );
            assert_eq!(
                formatted["formatNodes"][1]["value"]["layout"], expected_layout,
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][0]["value"]["formatterProfile"], profile,
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][0]["writerKind"], "token",
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][2]["value"]["quoted"], false,
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][2]["value"]["sourceRange"]["byteOffset"], 0,
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][2]["value"]["sourceRange"]["byteLength"], 2,
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][2]["outputSpan"]["origin"],
                csv_test_source_map(0, 2),
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][3]["outputSpan"]["origin"],
                csv_test_source_map(2, 1),
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][4]["outputSpan"]["origin"],
                csv_test_source_map(3, 4),
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][9]["value"]["rowSourceRange"]["byteOffset"], 0,
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][9]["outputSpan"]["origin"],
                csv_test_source_map(20, 1),
                "{profile}"
            );
            assert_eq!(
                formatted["nodes"][9]["value"]["fieldCount"], 4,
                "{profile}"
            );
            assert_eq!(writer_node_text(&formatted), expected_output, "{profile}");
            let target = TransformTemplateEncodingTarget::new(
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                "csv-document",
            );
            let mut context =
                TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                    &target,
                    Some(TransformTemplateOutputProducedKind::CemTree),
                );
            context.formatter_profile = Some(profile.to_owned());
            context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
            context.canonical = Some(profile == "compact");
            context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
            let mut identity = TransformTemplateEncodedArtifactIdentity::new(
                TransformTemplateOutputProducedKind::CemTree,
                target,
            );
            identity.formatter_profile = Some(profile.to_owned());
            identity.mode = TransformTemplateEncodedArtifactMode::Document;
            identity.canonical = profile == "compact";
            identity.source_map_policy = TransformTemplateSourceMapPolicy::Generated;
            TransformTemplateEncodedArtifact::new(identity, formatted)
                .validate_insertion(&context)
                .unwrap_or_else(|error| panic!("{profile} CSV formatter CEM tree: {error:?}"));
            assert!(
                builtin_schema_package_artifact_source("csv", expected_path)
                    .is_some_and(|source| source.source.contains("{body |")),
                "{profile} formatter asset must be embedded with an executable body"
            );
        }
    }

    #[test]
    fn builtin_csv_tabular_formatter_applies_max_field_width_options() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let table = serde_json::json!({
            "kind": "csv-table",
            "rows": [
                {
                    "index": 0,
                    "fields": [
                        {"index": 0, "value": "id"},
                        {"index": 1, "value": "name"},
                        {"index": 2, "value": "total"}
                    ]
                },
                {
                    "index": 1,
                    "fields": [
                        {"index": 0, "value": "123456789"},
                        {"index": 1, "value": "Alexandria"},
                        {"index": 2, "value": "123.4567"}
                    ]
                },
                {
                    "index": 2,
                    "fields": [
                        {"index": 0, "value": "42"},
                        {"index": 1, "value": "Bo"},
                        {"index": 2, "value": "9.5"}
                    ]
                }
            ]
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            cemt_formatter_options: BTreeMap::from([
                ("csv.maxFieldWidth".to_owned(), "6".to_owned()),
                ("csv.stringTrim".to_owned(), "middle".to_owned()),
            ]),
            ..ScopeConfig::default()
        };

        let execution = execute_csv_document_output_pipeline_with_environment(
            &environment,
            table,
            &target_scope,
            Some("builtin:csv-tabular-options"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("id    ,name  ,total \n...789,Al...a,123.45\n    42,Bo    ,  9.5 \n")
        );
    }

    #[test]
    fn builtin_csv_formatter_applies_tab_size_options() {
        let (formatted, _execution) = execute_builtin_csv_formatter_profile_with_options(
            "pretty",
            BTreeMap::from([("tabSize".to_owned(), "6".to_owned())]),
        )
        .expect("CSV pretty formatter accepts generic tabSize option");

        assert_eq!(formatted["formatNodes"][1]["value"]["tabSize"], 6);
        assert_eq!(formatted["formatNodes"][2]["name"], "csv.presentation-plan");
        assert_eq!(formatted["formatNodes"][2]["value"]["tabSize"], 6);
    }

    #[test]
    fn builtin_csv_formatter_applies_line_ending_options() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let base_table = serde_json::json!({
            "kind": "csv-table",
            "rows": [
                {
                    "index": 0,
                    "fields": [
                        {"index": 0, "value": "id"},
                        {"index": 1, "value": "name"}
                    ]
                },
                {
                    "index": 1,
                    "fields": [
                        {"index": 0, "value": "1"},
                        {"index": 1, "value": "Ada"}
                    ]
                }
            ]
        });

        for (name, table, option, expected) in [
            (
                "explicit crlf",
                base_table.clone(),
                "crlf",
                "id,name\r\n1,Ada\r\n",
            ),
            ("explicit lf", base_table.clone(), "lf", "id,name\n1,Ada\n"),
            (
                "preserve crlf",
                serde_json::json!({
                    "kind": "csv-table",
                    "lineEnding": "crlf",
                    "rows": base_table["rows"].clone()
                }),
                "preserve",
                "id,name\r\n1,Ada\r\n",
            ),
            (
                "preserve fallback",
                base_table.clone(),
                "preserve",
                "id,name\n1,Ada\n",
            ),
        ] {
            let target_scope = ScopeConfig {
                cemt_formatter_profile: Some("compact".to_owned()),
                cemt_formatter_options: BTreeMap::from([(
                    "lineEnding".to_owned(),
                    option.to_owned(),
                )]),
                ..ScopeConfig::default()
            };

            let execution = execute_csv_document_output_pipeline_with_environment(
                &environment,
                table,
                &target_scope,
                Some("builtin:csv-line-ending-options"),
            );

            assert!(
                execution.diagnostics.is_empty(),
                "{name}: {:?}",
                execution.diagnostics
            );
            assert_eq!(
                execution.output.as_ref().and_then(Value::as_str),
                Some(expected),
                "{name}"
            );
        }
    }

    #[test]
    fn builtin_csv_output_pipeline_renders_html_color_backend_with_terminal_text_parity() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let table = serde_json::json!({
            "kind": "csv-table",
            "rows": [
                {
                    "index": 0,
                    "fields": [
                        {"index": 0, "value": "id"},
                        {"index": 1, "value": "name"},
                        {"index": 2, "value": "active"}
                    ]
                },
                {
                    "index": 1,
                    "fields": [
                        {"index": 0, "value": "1"},
                        {"index": 1, "value": "Ada"},
                        {"index": 2, "value": "true"}
                    ]
                },
                {
                    "index": 2,
                    "fields": [
                        {"index": 0, "value": "2"},
                        {"index": 1, "value": "Lin"},
                        {"index": 2, "value": "false"}
                    ]
                }
            ]
        });
        let terminal_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            output_color_type: Some("ansi-256".to_owned()),
            ..ScopeConfig::default()
        };
        let html_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            output_color_type: Some("html-css-vars".to_owned()),
            ..ScopeConfig::default()
        };
        let html_color_profile_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            cemt_color_profile: Some("html".to_owned()),
            ..ScopeConfig::default()
        };
        let html_tab_size_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            cemt_formatter_options: BTreeMap::from([("tabSize".to_owned(), "6".to_owned())]),
            output_color_type: Some("html-css-vars".to_owned()),
            ..ScopeConfig::default()
        };
        let plain_scope = ScopeConfig {
            cemt_formatter_profile: Some("tabular".to_owned()),
            cemt_color_profile: Some("terminal".to_owned()),
            output_color_type: Some("none".to_owned()),
            ..ScopeConfig::default()
        };

        let terminal = execute_csv_document_output_pipeline_with_environment(
            &environment,
            table.clone(),
            &terminal_scope,
            Some("builtin:csv-terminal-output"),
        );
        let html = execute_csv_document_output_pipeline_with_environment(
            &environment,
            table.clone(),
            &html_scope,
            Some("builtin:csv-html-output"),
        );
        let html_color_profile = execute_csv_document_output_pipeline_with_environment(
            &environment,
            table.clone(),
            &html_color_profile_scope,
            Some("builtin:csv-html-color-profile-output"),
        );
        let html_tab_size = execute_csv_document_output_pipeline_with_environment(
            &environment,
            table.clone(),
            &html_tab_size_scope,
            Some("builtin:csv-html-tab-size-output"),
        );
        let plain = execute_csv_document_output_pipeline_with_environment(
            &environment,
            table,
            &plain_scope,
            Some("builtin:csv-plain-output"),
        );

        assert!(
            terminal.diagnostics.is_empty(),
            "{:?}",
            terminal.diagnostics
        );
        assert!(html.diagnostics.is_empty(), "{:?}", html.diagnostics);
        assert!(
            html_color_profile.diagnostics.is_empty(),
            "{:?}",
            html_color_profile.diagnostics
        );
        assert!(
            html_tab_size.diagnostics.is_empty(),
            "{:?}",
            html_tab_size.diagnostics
        );
        assert!(plain.diagnostics.is_empty(), "{:?}", plain.diagnostics);
        assert!(terminal.format_elapsed_ns.is_some());
        assert!(terminal.color_elapsed_ns.is_some());
        assert!(terminal.writer_elapsed_ns.is_some());
        assert!(plain.format_elapsed_ns.is_some());
        assert!(plain.color_elapsed_ns.is_some());
        assert!(plain.writer_elapsed_ns.is_some());
        let terminal_text =
            strip_ansi_codes(terminal.output.as_ref().and_then(Value::as_str).unwrap());
        let html_output = html.output.as_ref().and_then(Value::as_str).unwrap();
        let html_color_profile_output = html_color_profile
            .output
            .as_ref()
            .and_then(Value::as_str)
            .unwrap();
        let plain_output = plain.output.as_ref().and_then(Value::as_str).unwrap();
        assert!(
            html_output.starts_with(&csv_html_preview_prefix(default_formatter_tab_size())),
            "{html_output}"
        );
        assert!(
            html_color_profile_output
                .starts_with(&csv_html_preview_prefix(default_formatter_tab_size())),
            "{html_color_profile_output}"
        );
        assert!(
            html_tab_size
                .output
                .as_ref()
                .and_then(Value::as_str)
                .is_some_and(|output| output.starts_with(&csv_html_preview_prefix(6))),
            "{:?}",
            html_tab_size.output
        );
        assert!(
            html_output.contains(r#"data-role="data.field.1""#),
            "{html_output}"
        );
        assert!(
            html_output.contains(r#"style="color: var(--cem-color-data-field-1, "#),
            "{html_output}"
        );
        assert!(html_output.ends_with("</pre>\n"), "{html_output}");
        assert!(
            html_color_profile_output.ends_with("</pre>\n"),
            "{html_color_profile_output}"
        );
        assert_eq!(
            html_text_content(html_output.strip_suffix('\n').unwrap_or(html_output)),
            terminal_text
        );
        assert_eq!(
            html_text_content(
                html_color_profile_output
                    .strip_suffix('\n')
                    .unwrap_or(html_color_profile_output)
            ),
            terminal_text
        );
        assert!(
            html_color_profile_output.contains(r#"style="color: var(--cem-color-data-field-1, "#),
            "{html_color_profile_output}"
        );
        assert!(!plain_output.contains('\u{1b}'), "{plain_output}");
        assert_eq!(plain_output, terminal_text);
    }

    #[test]
    fn builtin_csv_output_pipeline_generates_output_spans_from_formatter_tokens() {
        let schema_registry = SchemaRegistry::with_builtin_schemas();
        let conversion_registry = ConversionRegistry::with_builtin_converters();
        let environment = ConversionOutputPipelineEnvironment {
            schema_registry: &schema_registry,
            conversion_registry: &conversion_registry,
            package_artifact_reader: None,
            artifact_cache: None,
        };
        let table = serde_json::json!({
            "kind": "csv-table",
            "rows": [{
                "index": 0,
                "recordEndingSourceMap": csv_test_source_map(7, 1),
                "fields": [
                    {
                        "index": 0,
                        "value": "id",
                        "sourceMap": csv_test_source_map(0, 2)
                    },
                    {
                        "index": 1,
                        "value": "name",
                        "delimiterBeforeSourceMap": csv_test_source_map(2, 1),
                        "sourceMap": csv_test_source_map(3, 4)
                    }
                ]
            }]
        });
        let target_scope = ScopeConfig {
            cemt_formatter_profile: Some("compact".to_owned()),
            ..ScopeConfig::default()
        };

        let execution = execute_csv_document_output_pipeline_with_environment(
            &environment,
            table,
            &target_scope,
            Some("builtin:csv-output-spans"),
        );

        assert!(
            execution.diagnostics.is_empty(),
            "{:?}",
            execution.diagnostics
        );
        assert_eq!(
            execution.output.as_ref().and_then(Value::as_str),
            Some("id,name\n")
        );
        assert_eq!(execution.output_spans.len(), 4);
        for (index, (output_start, output_len, source_start, source_len)) in [
            (0u64, 2u32, 0u64, 2u32),
            (2u64, 1u32, 2u64, 1u32),
            (3u64, 4u32, 3u64, 4u32),
            (7u64, 1u32, 7u64, 1u32),
        ]
        .into_iter()
        .enumerate()
        {
            let span = &execution.output_spans[index];
            assert_eq!(span.output_range.start, output_start, "span {index}");
            assert_eq!(span.output_range.len, output_len, "span {index}");
            let frame = span
                .origin
                .frames
                .first()
                .unwrap_or_else(|| panic!("span {index} has origin frame"));
            assert_eq!(frame.source_id, SourceId(0), "span {index}");
            match frame.span {
                crate::source_map::FrameSpan::Single(range) => {
                    assert_eq!(range.start, source_start, "span {index}");
                    assert_eq!(range.len, source_len, "span {index}");
                }
                crate::source_map::FrameSpan::Multi(_) => {
                    panic!("span {index} should have a single source range")
                }
            }
        }
    }

    #[test]
    fn builtin_csv_colorizer_profiles_execute_package_cemt_assets() {
        for (profile, expected_path, expected_output, expected_style_key, expected_style_value) in [
            (
                "terminal",
                "schema-packages/csv/v1/colorizers/terminal.cemt",
                "terminal",
                "terminalCapability",
                "auto",
            ),
            (
                "html",
                "schema-packages/csv/v1/colorizers/html.cemt",
                "html",
                "htmlMode",
                "classes",
            ),
            (
                "md",
                "schema-packages/csv/v1/colorizers/md.cemt",
                "md",
                "wrapper",
                "span",
            ),
        ] {
            let (colored, execution, binding) = execute_builtin_csv_colorizer_profile(profile)
                .unwrap_or_else(|error| panic!("{profile} CSV colorizer failed: {error}"));

            assert_eq!(
                execution,
                ConversionOutputPipelineStageExecution::CemtAdapter {
                    adapter_id: "csv-color-cemt".to_owned(),
                    function_name: "csv.color-document".to_owned(),
                    body_function_name: Some("csv.color-document".to_owned()),
                    fallback_function_name: None,
                },
                "{profile}"
            );
            assert_eq!(
                binding.function.profile.as_deref(),
                Some(profile),
                "{profile}"
            );
            assert_eq!(
                binding.identity.color_profile.as_deref(),
                Some(profile),
                "{profile}"
            );
            assert_eq!(colored["kind"], "cem-tree", "{profile}");
            assert_eq!(colored["formatterProfile"], "compact", "{profile}");
            assert_eq!(colored["colorProfile"], profile, "{profile}");
            assert_eq!(colored["colorOutput"], expected_output, "{profile}");
            assert_eq!(colored["colored"], true, "{profile}");
            assert_eq!(writer_node_text(&colored), "id,name\n", "{profile}");
            assert_eq!(
                colored["colorNodes"][0]["name"], "csv.color-document",
                "{profile}"
            );
            assert_eq!(
                colored["nodes"][0]["outputSpan"],
                csv_test_output_span(0, 2),
                "{profile}"
            );
            assert_eq!(colored["nodes"][0]["value"]["rowIndex"], 0, "{profile}");
            assert_eq!(
                colored["nodes"][0]["value"]["formatterProfile"], "compact",
                "{profile}"
            );
            assert_eq!(
                colored["nodes"][0]["value"]["colorProfile"], profile,
                "{profile}"
            );
            let expected_first_field_role = if profile == "md" {
                "syntax.string"
            } else {
                "data.field.1"
            };
            assert_eq!(
                colored["nodes"][0]["value"]["colorRole"], expected_first_field_role,
                "{profile}"
            );
            if profile == "terminal" || profile == "html" {
                assert_eq!(
                    colored["nodes"][2]["style"]["colorRole"], "data.field.2",
                    "{profile}"
                );
            }
            assert_eq!(
                colored["nodes"][1]["style"]["colorRole"], "syntax.punctuation",
                "{profile}"
            );
            assert_eq!(
                colored["nodes"][1]["style"][expected_style_key], expected_style_value,
                "{profile}"
            );
            let target = TransformTemplateEncodingTarget::new(
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                "csv-document",
            );
            let mut context =
                TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                    &target,
                    Some(TransformTemplateOutputProducedKind::CemTree),
                );
            context.formatter_profile = Some("compact".to_owned());
            context.color_profile = Some(profile.to_owned());
            context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
            context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
            binding
                .artifact_from_value(colored)
                .validate_insertion(&context)
                .unwrap_or_else(|error| panic!("{profile} CSV colorized CEM tree: {error:?}"));
            assert!(
                builtin_schema_package_artifact_source("csv", expected_path)
                    .is_some_and(|source| source.source.contains("{body |")),
                "{profile} colorizer asset must be embedded with an executable body"
            );
        }
    }

    #[test]
    fn cemt_output_stage_executes_direct_colorizer_body_expression() {
        let stage = CemTreeCemtOutputStage {
            adapter_id: "cem-tree-color-direct-cemt",
            package_id: "cem-ml".to_owned(),
            target: TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
            stage_profile: Some("classes".to_owned()),
            template_uri: "builtin:cem.color-tree.direct.cemt".to_owned(),
            template_bytes: r#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {color-function
        @name="cem.color-tree"
        @category="cem-tree"
        @subject="cem-tree"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="css-custom-properties"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {body |
            {$ {
                kind: $subject.kind,
                contentType: $subject.contentType,
                schema: $subject.schema,
                category: $subject.category,
                mode: $subject.mode,
                canonical: $subject.canonical,
                formatterProfile: $subject.formatterProfile,
                formatNodes: $subject.formatNodes,
                colored: true,
                colorProfile: "none",
                colorNodes: [
                    {
                        kind: "color-marker",
                        name: "cem.color-tree",
                        colorizerRole: "colorizer.boundary",
                        colorProfile: "none"
                    },
                    {
                        kind: "color-decision",
                        name: "profile",
                        colorizerRole: "colorizer.profile",
                        value: "none",
                        colorProfile: "none"
                    }
                ],
                nodes: map($subject.nodes, {
                    kind: $item.kind,
                    name: $item.name,
                    colorRole: "syntax.name",
                    writerAttributeNodes: [
                        {
                            kind: "writer-attribute",
                            name: "class",
                            value: "direct-cemt-color",
                            colorProfile: "none",
                            colorizerOwned: true,
                            colorizerRole: "colorizer.writer-attribute"
                        }
                    ],
                    children: $item.children
                })
            } }
        }
    }
}
"#
            .as_bytes()
            .to_vec(),
            declaration_element: "{color-function",
            function_kind: TransformTemplateOutputFunctionKind::Color,
            function_name: "cem.color-tree".to_owned(),
            canonical_function_name: "cem.color-tree",
            role: "colorizer",
        };
        let formatted = serde_json::json!({
            "kind": "cem-tree",
            "contentType": CEM_ML_CONTENT_TYPE,
            "schema": CEM_ML_SCHEMA_URI,
            "category": "cem-tree",
            "mode": "document",
            "canonical": true,
            "formatterProfile": "compact",
            "formatNodes": [
                {
                    "kind": "format-marker",
                    "name": "cem.format-tree",
                    "formatterRole": "formatter.boundary",
                    "formatterProfile": "compact"
                },
                {
                    "kind": "format-decision",
                    "name": "layout",
                    "formatterRole": "formatter.layout",
                    "value": "direct-cemt",
                    "formatterProfile": "compact"
                }
            ],
            "nodes": [{
                "kind": "element",
                "name": "main",
                "children": [{"kind": "text", "value": "Ready"}]
            }]
        });
        let mut registry = TransformTemplateOutputFunctionRegistry::new();
        registry.register(conversion_cem_tree_color_function_descriptor("none"));
        let request = TransformTemplateEncodeBindingRequest::new(
            formatted.clone(),
            TransformTemplateEncodingTarget::new(
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                "cem-tree",
            ),
        )
        .with_subject_type("cem-tree")
        .with_options(TransformTemplateEncodeOptions {
            colorizer: Some("cem.color-tree".to_owned()),
            color_profile: Some("none".to_owned()),
            canonical: true,
            ..TransformTemplateEncodeOptions::default()
        });
        let binding = registry
            .resolve_color_binding(&request, &BTreeSet::new())
            .expect("direct colorizer binding resolves")
            .into_encode_binding();

        let (colored, execution) =
            execute_test_conversion_cem_tree_output_stage(stage, &binding, &formatted)
                .expect("direct colorizer body runs");

        assert_eq!(
            execution,
            ConversionOutputPipelineStageExecution::CemtAdapter {
                adapter_id: "cem-tree-color-direct-cemt".to_owned(),
                function_name: "cem.color-tree".to_owned(),
                body_function_name: Some("cem.color-tree".to_owned()),
                fallback_function_name: None,
            }
        );
        assert_eq!(colored["kind"], "cem-tree");
        assert_eq!(colored["colored"], true);
        assert_eq!(colored["colorProfile"], "none");
        assert_eq!(colored["formatNodes"], formatted["formatNodes"]);
        assert_eq!(colored["colorNodes"][0]["name"], "cem.color-tree");
        assert_eq!(
            colored["colorNodes"][1]["colorizerRole"],
            "colorizer.profile"
        );
        assert_eq!(colored["nodes"][0]["name"], "main");
        assert_eq!(colored["nodes"][0]["colorRole"], "syntax.name");
        assert_eq!(
            colored["nodes"][0]["writerAttributeNodes"][0]["value"],
            "direct-cemt-color"
        );

        binding
            .artifact_from_value(colored)
            .validate_insertion(
                &TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
                    &request.target,
                    Some(TransformTemplateOutputProducedKind::CemTree),
                ),
            )
            .expect("direct colored CEM tree validates");
    }

    #[test]
    fn builtin_registry_is_loaded_from_package_manifest_converters() {
        let registry = ConversionRegistry::with_builtin_converters();
        let ids = registry
            .converters()
            .map(|descriptor| descriptor.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), 12);
        for id in [
            "cem-ml-to-dom-projection-rust",
            "cem-ml-to-ast-projection-rust",
            "cem-ml-to-events-projection-rust",
            "html-to-cem-dom-projection-rust",
            "xml-to-cem-dom-projection-rust",
            "cem-dom-projection-to-html-cemt",
            "cem-dom-projection-to-xml-cemt",
            "cem-dom-projection-to-html-rust",
            "cem-dom-projection-to-xml-rust",
            "cem-dom-projection-to-json-debug-rust",
            "cem-ast-projection-to-json-debug-rust",
            "cem-events-projection-to-json-debug-rust",
        ] {
            assert!(ids.contains(id), "missing built-in converter `{id}`");
        }

        let html = registry
            .converter("html-to-cem-dom-projection-rust")
            .expect("HTML recovery converter");
        assert_eq!(html.package_id, "html");
        assert_eq!(html.cost, 50);
        assert_eq!(html.lossiness.as_deref(), Some("recovery"));

        let xml = registry
            .converter("xml-to-cem-dom-projection-rust")
            .expect("XML DOM projection converter");
        assert_eq!(xml.package_id, "xml");
        assert_eq!(xml.cost, 80);

        let ast_debug = registry
            .converter("cem-ast-projection-to-json-debug-rust")
            .expect("AST JSON debug converter");
        assert_eq!(ast_debug.from.content_type, CEM_AST_PROJECTION_CONTENT_TYPE);
        assert_eq!(
            ast_debug.to.content_type,
            CEM_AST_JSON_PROJECTION_CONTENT_TYPE
        );

        let events_debug = registry
            .converter("cem-events-projection-to-json-debug-rust")
            .expect("events JSON debug converter");
        assert_eq!(
            events_debug.from.content_type,
            CEM_EVENTS_PROJECTION_CONTENT_TYPE
        );
        assert_eq!(
            events_debug.to.content_type,
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE
        );
    }

    #[test]
    fn builtin_registry_classifies_conversion_planning_domains() {
        let registry = ConversionRegistry::with_builtin_converters();

        for (id, expected_domain) in [
            (
                "cem-ml-to-dom-projection-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "cem-ml-to-ast-projection-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "cem-ml-to-events-projection-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "html-to-cem-dom-projection-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "xml-to-cem-dom-projection-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "cem-dom-projection-to-json-debug-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "cem-ast-projection-to-json-debug-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "cem-events-projection-to-json-debug-rust",
                ConversionPlanningDomain::ContentTypeConversion,
            ),
            (
                "cem-dom-projection-to-html-cemt",
                ConversionPlanningDomain::SchemaOutputProduction,
            ),
            (
                "cem-dom-projection-to-xml-cemt",
                ConversionPlanningDomain::SchemaOutputProduction,
            ),
            (
                "cem-dom-projection-to-html-rust",
                ConversionPlanningDomain::SchemaOutputProduction,
            ),
            (
                "cem-dom-projection-to-xml-rust",
                ConversionPlanningDomain::SchemaOutputProduction,
            ),
        ] {
            let descriptor = registry.converter(id).expect("built-in converter");
            assert_eq!(descriptor.planning_domain(), expected_domain, "{id} domain");
        }
    }

    #[test]
    fn content_conversion_and_schema_output_lookup_are_separate() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let registry = ConversionRegistry::with_builtin_converters();

        let content_conversion = registry
            .select_content_type_conversion_edge(
                &schemas,
                &identity(HTML_CONTENT_TYPE),
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
            )
            .unwrap();
        assert_eq!(
            content_conversion.descriptor.id,
            "html-to-cem-dom-projection-rust"
        );
        assert_eq!(
            content_conversion.descriptor.planning_domain(),
            ConversionPlanningDomain::ContentTypeConversion
        );

        let content_output_attempt = registry
            .select_content_type_conversion_edge(
                &schemas,
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
                &identity(HTML_CONTENT_TYPE),
            )
            .unwrap_err();
        assert!(matches!(
            content_output_attempt,
            ConversionLookupError::NoDirectEdge { .. }
        ));

        let schema_output = registry
            .select_schema_output_producer(
                &schemas,
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
                &identity(HTML_CONTENT_TYPE),
            )
            .unwrap();
        assert_eq!(
            schema_output.descriptor.id,
            "cem-dom-projection-to-html-cemt"
        );
        assert_eq!(
            schema_output.descriptor.planning_domain(),
            ConversionPlanningDomain::SchemaOutputProduction
        );

        let schema_input_attempt = registry
            .select_schema_output_producer(
                &schemas,
                &identity(HTML_CONTENT_TYPE),
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
            )
            .unwrap_err();
        assert!(matches!(
            schema_input_attempt,
            ConversionLookupError::NoDirectEdge { .. }
        ));
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
    fn builtin_execution_resolves_ready_cemt_to_rust_fallback_when_adapter_is_selector_only() {
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
                assert!(reason.contains("executable CEMT adapter"));
                assert!(reason.contains("selector-only"));
                assert_eq!(*template_adapter_id, Some("cem-native-template"));
            }
            other => panic!("expected Rust fallback execution, got {other:?}"),
        }
    }

    #[test]
    fn default_engine_context_resolves_builtin_dom_cemt_converter_as_template() {
        let context = EngineContext::default();

        let execution = context
            .converter_registry
            .resolve_direct_execution(
                &context.schema_registry,
                &context.template_adapter_registry,
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
                &identity(HTML_CONTENT_TYPE),
            )
            .unwrap();

        assert_eq!(execution.descriptor.id, "cem-dom-projection-to-html-cemt");
        match &execution.execution {
            ConversionExecution::CemtTemplate {
                adapter_id,
                template,
            } => {
                assert_eq!(*adapter_id, "dom-projection-parity-cemt");
                assert_eq!(template.entrypoint.as_deref(), Some("main"));
                assert!(template.path.ends_with("converters/dom-to-html.cemt"));
            }
            other => panic!("expected CEMT template execution, got {other:?}"),
        }
    }

    #[test]
    fn domain_scoped_execution_uses_separate_paths() {
        let schemas = SchemaRegistry::with_builtin_schemas();
        let registry = ConversionRegistry::with_builtin_converters();
        let template_adapters = TransformTemplateAdapterRegistry::with_builtin_adapters();

        let content_execution = registry
            .resolve_content_type_conversion_execution(
                &schemas,
                &template_adapters,
                &identity(HTML_CONTENT_TYPE),
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
            )
            .unwrap();
        assert_eq!(
            content_execution.descriptor.id,
            "html-to-cem-dom-projection-rust"
        );
        assert_eq!(
            content_execution.execution,
            ConversionExecution::Rust {
                rust_symbol: "Html5RecoveryConverter".to_owned()
            }
        );

        let schema_output_execution = registry
            .resolve_schema_output_execution(
                &schemas,
                &template_adapters,
                &identity(CEM_DOM_PROJECTION_CONTENT_TYPE),
                &identity(HTML_CONTENT_TYPE),
            )
            .unwrap();
        assert_eq!(
            schema_output_execution.descriptor.id,
            "cem-dom-projection-to-html-cemt"
        );
        match schema_output_execution.execution {
            ConversionExecution::RustFallback { rust_symbol, .. } => {
                assert_eq!(rust_symbol, "HtmlExportConverter");
            }
            other => panic!("expected schema output Rust fallback, got {other:?}"),
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
                    entrypoint: Some("main".to_owned()),
                }),
                rust_symbol: None,
                rust_fallback: None,
                streamable: true,
                lossiness: Some("serialization".to_owned()),
                output_contract: ConversionOutputContractDescriptor::default(),
                parity_fixtures: Vec::new(),
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
