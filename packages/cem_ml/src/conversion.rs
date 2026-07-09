//! Schema-owned content conversion registry.
//!
//! This module is the planning and dispatch-side contract for schema package
//! converter edges. Runtime execution still flows through the existing
//! lifecycle and transform-template adapters.

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
use crate::schema::package_sources::builtin_schema_package_artifact_source;
use crate::schema::registry::{
    content_type_essence, SchemaContentTypeRole, SchemaDescriptor, SchemaRegistry,
    CEM_AST_PROJECTION_SCHEMA_URI, CEM_DOM_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
    CEM_EVENTS_PROJECTION_SCHEMA_URI, CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI,
    CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, HTML_CONTENT_TYPE, HTML_SCHEMA_URI,
    JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI, XML_CONTENT_TYPE, XML_SCHEMA_URI, YAML_CONTENT_TYPE,
    YAML_SCHEMA_URI,
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
    parse_cem_native_template_module_options,
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
    TransformTemplateEncodedArtifact, TransformTemplateEncodedArtifactInsertionContext,
    TransformTemplateEncodedArtifactMode, TransformTemplateEncodingTarget,
    TransformTemplateEvaluatedEncodeExpression, TransformTemplateHtmlColorMode,
    TransformTemplateModuleOptions, TransformTemplateModuleParseRequest,
    TransformTemplateModulePreflight, TransformTemplateModuleVisibility,
    TransformTemplateOutputArtifact, TransformTemplateOutputFunctionDescriptor,
    TransformTemplateOutputFunctionImplementation, TransformTemplateOutputFunctionKind,
    TransformTemplateOutputFunctionRegistry, TransformTemplateOutputProducedKind,
    TransformTemplateRenderRequest, TransformTemplateRenderResponse,
    TransformTemplateSourceMapPolicy, TransformTemplateTargetSyntaxKind,
    TransformTemplateTargetSyntaxRules, TransformTemplateTerminalColorCapability,
};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

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
pub const CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE: &str =
    "cem.converter.output_pipeline_execution";

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
    MissingAttribute {
        converter_id: Option<String>,
        attribute: &'static str,
    },
    MissingEndpoint {
        converter_id: String,
        endpoint: &'static str,
    },
    UnknownImplementation {
        converter_id: String,
        implementation: String,
    },
    UnknownReadiness {
        converter_id: String,
        readiness: String,
    },
    UnknownOutputSyntax {
        converter_id: String,
        output_syntax: String,
    },
    UnknownParityMode {
        converter_id: String,
        parity: String,
    },
    InvalidBoolean {
        converter_id: String,
        attribute: &'static str,
        value: String,
    },
    InvalidCost {
        converter_id: String,
        value: String,
    },
    ArtifactContract {
        package_id: String,
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
            Self::MissingAttribute {
                converter_id,
                attribute,
            } => {
                if let Some(converter_id) = converter_id {
                    write!(
                        f,
                        "converter `{converter_id}` is missing required attribute `{attribute}`"
                    )
                } else {
                    write!(f, "converter is missing required attribute `{attribute}`")
                }
            }
            Self::MissingEndpoint {
                converter_id,
                endpoint,
            } => write!(
                f,
                "converter `{converter_id}` is missing required `{endpoint}` endpoint"
            ),
            Self::UnknownImplementation {
                converter_id,
                implementation,
            } => write!(
                f,
                "converter `{converter_id}` has unknown implementation `{implementation}`"
            ),
            Self::UnknownReadiness {
                converter_id,
                readiness,
            } => write!(
                f,
                "converter `{converter_id}` has unknown readiness `{readiness}`"
            ),
            Self::UnknownOutputSyntax {
                converter_id,
                output_syntax,
            } => write!(
                f,
                "converter `{converter_id}` has unknown output syntax `{output_syntax}`"
            ),
            Self::UnknownParityMode {
                converter_id,
                parity,
            } => write!(
                f,
                "converter `{converter_id}` has unknown parity mode `{parity}`"
            ),
            Self::InvalidBoolean {
                converter_id,
                attribute,
                value,
            } => write!(
                f,
                "converter `{converter_id}` has invalid boolean `{attribute}` value `{value}`"
            ),
            Self::InvalidCost {
                converter_id,
                value,
            } => write!(
                f,
                "converter `{converter_id}` has invalid cost value `{value}`"
            ),
            Self::ArtifactContract {
                package_id,
                path,
                message,
            } => write!(
                f,
                "schema package `{package_id}` artifact `{path}` has invalid CEMT contract: {message}"
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericDataTextDocument {
    pub content: String,
    pub content_type: String,
    pub schema: String,
}

#[derive(Debug, Clone)]
pub enum GenericDataTextConversionOutcome {
    Unsupported,
    Converted {
        document: GenericDataTextDocument,
        diagnostics: Vec<Diagnostic>,
    },
    Failed {
        diagnostics: Vec<Diagnostic>,
    },
}

#[derive(Debug, Clone)]
pub struct GenericDataValueAst {
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum GenericDataTextAstReadOutcome {
    Unsupported,
    Read {
        ast: GenericDataValueAst,
        diagnostics: Vec<Diagnostic>,
    },
    Failed {
        diagnostics: Vec<Diagnostic>,
    },
}

#[derive(Debug, Clone)]
pub enum GenericDataTextAstWriteOutcome {
    Unsupported,
    Written {
        document: GenericDataTextDocument,
        diagnostics: Vec<Diagnostic>,
    },
    Failed {
        diagnostics: Vec<Diagnostic>,
    },
}

pub fn convert_generic_data_text(
    schema_registry: &SchemaRegistry,
    source: &FormatIdentity,
    target: &FormatIdentity,
    uri: Option<&str>,
    bytes: &[u8],
) -> GenericDataTextConversionOutcome {
    if !generic_data_text_source_is_supported(schema_registry, source)
        || !generic_data_text_target_is_supported(schema_registry, target)
    {
        return GenericDataTextConversionOutcome::Unsupported;
    }

    let (ast, mut diagnostics) =
        match read_generic_data_text_ast(schema_registry, source, uri, bytes) {
            GenericDataTextAstReadOutcome::Unsupported => {
                return GenericDataTextConversionOutcome::Unsupported;
            }
            GenericDataTextAstReadOutcome::Read { ast, diagnostics } => (ast, diagnostics),
            GenericDataTextAstReadOutcome::Failed { diagnostics } => {
                return GenericDataTextConversionOutcome::Failed { diagnostics };
            }
        };

    match write_generic_data_text_ast(schema_registry, target, &ast) {
        GenericDataTextAstWriteOutcome::Unsupported => {
            GenericDataTextConversionOutcome::Unsupported
        }
        GenericDataTextAstWriteOutcome::Written {
            document,
            diagnostics: mut write_diagnostics,
        } => {
            diagnostics.append(&mut write_diagnostics);
            GenericDataTextConversionOutcome::Converted {
                document,
                diagnostics,
            }
        }
        GenericDataTextAstWriteOutcome::Failed {
            diagnostics: mut write_diagnostics,
        } => {
            diagnostics.append(&mut write_diagnostics);
            GenericDataTextConversionOutcome::Failed { diagnostics }
        }
    }
}

pub fn read_generic_data_text_ast(
    schema_registry: &SchemaRegistry,
    source: &FormatIdentity,
    uri: Option<&str>,
    bytes: &[u8],
) -> GenericDataTextAstReadOutcome {
    if format_identity_matches_schema(schema_registry, source, YAML_SCHEMA_URI) {
        return read_yaml_text_ast(uri, bytes);
    }

    if format_identity_matches_schema(schema_registry, source, JSON_VALUE_SCHEMA_URI) {
        return read_json_text_ast(uri, bytes);
    }

    GenericDataTextAstReadOutcome::Unsupported
}

pub fn write_generic_data_text_ast(
    schema_registry: &SchemaRegistry,
    target: &FormatIdentity,
    ast: &GenericDataValueAst,
) -> GenericDataTextAstWriteOutcome {
    if format_identity_matches_schema(schema_registry, target, JSON_VALUE_SCHEMA_URI) {
        return write_json_text_ast(target, ast);
    }

    if format_identity_matches_schema(schema_registry, target, YAML_SCHEMA_URI) {
        return write_yaml_text_ast(target, ast);
    }

    GenericDataTextAstWriteOutcome::Unsupported
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

/// Executable parity adapter for the packaged DOM-projection CEMT serializers.
///
/// This is a bounded adapter for conversion parity verification. It recognizes
/// the current packaged DOM serializer templates and executes their fixture
/// behavior; it is not a general-purpose CEMT interpreter.
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
        let output = if template.contains(r#"node.kind = "cdata""#) {
            "xml"
        } else if template.contains(r#"node.kind = "raw-text""#) {
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
            let tree =
                conversion_dom_projection_parity_cem_tree_document(&request.primary_input.value)
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
    let children = input
        .get("children")
        .and_then(Value::as_array)
        .ok_or_else(|| "DOM projection input must contain a children array".to_owned())?;
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

fn conversion_dom_projection_parity_cem_tree_document(input: &Value) -> Result<Value, String> {
    let children = input
        .get("children")
        .and_then(Value::as_array)
        .ok_or_else(|| "DOM projection input must contain a children array".to_owned())?;
    let mut nodes = Vec::new();
    for child in children {
        if let Some(node) = conversion_dom_projection_parity_cem_tree_node(child)? {
            nodes.push(node);
        }
    }
    Ok(Value::Array(nodes))
}

fn conversion_dom_projection_parity_cem_tree_node(node: &Value) -> Result<Option<Value>, String> {
    let kind = node.get("kind").and_then(Value::as_str).unwrap_or_default();
    match kind {
        "text" | "whitespace" | "comment" | "cdata" | "raw-text" => Ok(Some(serde_json::json!({
            "kind": kind,
            "value": conversion_dom_projection_parity_data(node),
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
    if !name.namespace.is_empty() {
        fields.insert(
            "namespace".to_owned(),
            Value::String(name.namespace.to_owned()),
        );
    }

    if let Some(attributes) = node.get("attributes").and_then(Value::as_array) {
        let attributes = attributes
            .iter()
            .map(conversion_dom_projection_parity_cem_tree_attribute)
            .collect::<Result<Vec<_>, _>>()?;
        if !attributes.is_empty() {
            fields.insert("attributes".to_owned(), Value::Array(attributes));
        }
    }

    if let Some(children) = node.get("children").and_then(Value::as_array) {
        let mut child_nodes = Vec::new();
        for child in children {
            if let Some(child_node) = conversion_dom_projection_parity_cem_tree_node(child)? {
                child_nodes.push(child_node);
            }
        }
        if !child_nodes.is_empty() {
            fields.insert("children".to_owned(), Value::Array(child_nodes));
        }
    }

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
    Ok(Value::Object(fields))
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

fn generic_data_text_source_is_supported(
    schema_registry: &SchemaRegistry,
    source: &FormatIdentity,
) -> bool {
    format_identity_matches_schema(schema_registry, source, YAML_SCHEMA_URI)
        || format_identity_matches_schema(schema_registry, source, JSON_VALUE_SCHEMA_URI)
}

fn generic_data_text_target_is_supported(
    schema_registry: &SchemaRegistry,
    target: &FormatIdentity,
) -> bool {
    format_identity_matches_schema(schema_registry, target, JSON_VALUE_SCHEMA_URI)
        || format_identity_matches_schema(schema_registry, target, YAML_SCHEMA_URI)
}

fn read_yaml_text_ast(uri: Option<&str>, bytes: &[u8]) -> GenericDataTextAstReadOutcome {
    let mut diagnostics = Vec::new();
    let source = match std::str::from_utf8(bytes) {
        Ok(source) => source,
        Err(error) => {
            diagnostics.push(generic_data_diagnostic(
                uri,
                None,
                None,
                "cem.yaml.unsupported_encoding",
                Severity::Error,
                format!("YAML source must be valid UTF-8: {error}"),
            ));
            return GenericDataTextAstReadOutcome::Failed { diagnostics };
        }
    };

    diagnostics.extend(collect_yaml_text_diagnostics(uri, source));
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return GenericDataTextAstReadOutcome::Failed { diagnostics };
    }

    let documents = match YamlLoader::load_from_str(source) {
        Ok(documents) => documents,
        Err(error) => {
            diagnostics.push(yaml_parse_diagnostic(uri, &error));
            return GenericDataTextAstReadOutcome::Failed { diagnostics };
        }
    };
    let value = yaml_documents_to_json_value(documents);

    GenericDataTextAstReadOutcome::Read {
        ast: GenericDataValueAst { value },
        diagnostics,
    }
}

fn read_json_text_ast(uri: Option<&str>, bytes: &[u8]) -> GenericDataTextAstReadOutcome {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(value) => GenericDataTextAstReadOutcome::Read {
            ast: GenericDataValueAst { value },
            diagnostics: Vec::new(),
        },
        Err(error) => GenericDataTextAstReadOutcome::Failed {
            diagnostics: vec![json_parse_diagnostic(uri, &error)],
        },
    }
}

fn write_json_text_ast(
    target: &FormatIdentity,
    ast: &GenericDataValueAst,
) -> GenericDataTextAstWriteOutcome {
    let content = match serde_json::to_string_pretty(&ast.value) {
        Ok(content) => content,
        Err(error) => {
            return GenericDataTextAstWriteOutcome::Failed {
                diagnostics: vec![generic_data_diagnostic(
                    None,
                    None,
                    None,
                    "cem.json.emit_error",
                    Severity::Error,
                    format!("JSON emit error: {error}"),
                )],
            };
        }
    };

    GenericDataTextAstWriteOutcome::Written {
        document: GenericDataTextDocument {
            content,
            content_type: target
                .content_type
                .clone()
                .unwrap_or_else(|| JSON_CONTENT_TYPE.to_owned()),
            schema: target
                .schema
                .clone()
                .unwrap_or_else(|| JSON_VALUE_SCHEMA_URI.to_owned()),
        },
        diagnostics: Vec::new(),
    }
}

fn write_yaml_text_ast(
    target: &FormatIdentity,
    ast: &GenericDataValueAst,
) -> GenericDataTextAstWriteOutcome {
    let yaml = json_value_to_yaml(&ast.value);
    let mut content = String::new();
    let mut emitter = YamlEmitter::new(&mut content);
    emitter.multiline_strings(true);
    if let Err(error) = emitter.dump(&yaml) {
        return GenericDataTextAstWriteOutcome::Failed {
            diagnostics: vec![generic_data_diagnostic(
                None,
                None,
                None,
                "cem.yaml.emit_error",
                Severity::Error,
                format!("YAML emit error: {error}"),
            )],
        };
    }

    GenericDataTextAstWriteOutcome::Written {
        document: GenericDataTextDocument {
            content,
            content_type: target
                .content_type
                .clone()
                .unwrap_or_else(|| YAML_CONTENT_TYPE.to_owned()),
            schema: target
                .schema
                .clone()
                .unwrap_or_else(|| YAML_SCHEMA_URI.to_owned()),
        },
        diagnostics: Vec::new(),
    }
}

fn format_identity_matches_schema(
    schema_registry: &SchemaRegistry,
    identity: &FormatIdentity,
    schema_uri: &str,
) -> bool {
    let explicit_schema_matches = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == schema_uri);

    let Some(content_type) = identity.content_type.as_deref() else {
        return explicit_schema_matches;
    };

    let Ok(descriptor) = schema_registry.resolve_content_type(content_type) else {
        return false;
    };
    descriptor.schema_uri == schema_uri && (identity.schema.is_none() || explicit_schema_matches)
}

fn yaml_documents_to_json_value(documents: Vec<Yaml>) -> Value {
    let mut values = documents
        .into_iter()
        .map(yaml_to_json_value)
        .collect::<Vec<_>>();
    if values.len() == 1 {
        values.remove(0)
    } else {
        Value::Array(values)
    }
}

fn yaml_to_json_value(value: Yaml) -> Value {
    match value {
        Yaml::Real(raw) => raw
            .replace('_', "")
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::String(raw)),
        Yaml::Integer(value) => Value::Number(value.into()),
        Yaml::String(value) => Value::String(value),
        Yaml::Boolean(value) => Value::Bool(value),
        Yaml::Array(values) => Value::Array(values.into_iter().map(yaml_to_json_value).collect()),
        Yaml::Hash(entries) => {
            let mut out = serde_json::Map::new();
            for (key, value) in entries {
                out.insert(yaml_key_to_json_key(key), yaml_to_json_value(value));
            }
            Value::Object(out)
        }
        Yaml::Alias(_) | Yaml::Null | Yaml::BadValue => Value::Null,
    }
}

fn yaml_key_to_json_key(key: Yaml) -> String {
    match key {
        Yaml::String(value) => value,
        Yaml::Integer(value) => value.to_string(),
        Yaml::Real(value) => value,
        Yaml::Boolean(value) => value.to_string(),
        Yaml::Null => "null".to_owned(),
        other => serde_json::to_string(&yaml_to_json_value(other)).unwrap_or_default(),
    }
}

fn json_value_to_yaml(value: &Value) -> Yaml {
    match value {
        Value::Null => Yaml::Null,
        Value::Bool(value) => Yaml::Boolean(*value),
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                Yaml::Integer(value)
            } else {
                Yaml::Real(number.to_string())
            }
        }
        Value::String(value) => Yaml::String(value.clone()),
        Value::Array(values) => Yaml::Array(values.iter().map(json_value_to_yaml).collect()),
        Value::Object(fields) => {
            let mut out = yaml_rust2::yaml::Hash::new();
            for (key, value) in fields {
                out.insert(Yaml::String(key.clone()), json_value_to_yaml(value));
            }
            Yaml::Hash(out)
        }
    }
}

fn json_parse_diagnostic(uri: Option<&str>, error: &serde_json::Error) -> Diagnostic {
    generic_data_diagnostic(
        uri,
        u32::try_from(error.line()).ok(),
        u32::try_from(error.column()).ok(),
        "cem.json.parse_error",
        Severity::Error,
        format!("JSON parse error: {error}"),
    )
}

fn yaml_parse_diagnostic(uri: Option<&str>, error: &yaml_rust2::scanner::ScanError) -> Diagnostic {
    let marker = error.marker();
    generic_data_diagnostic(
        uri,
        u32::try_from(marker.line()).ok(),
        u32::try_from(marker.col()).ok(),
        "cem.yaml.parse_error",
        Severity::Error,
        format!("YAML parse error: {error}"),
    )
}

fn collect_yaml_text_diagnostics(uri: Option<&str>, source: &str) -> Vec<Diagnostic> {
    let mut receiver = YamlDiagnosticReceiver {
        uri,
        diagnostics: Vec::new(),
    };
    let mut parser = yaml_rust2::parser::Parser::new_from_str(source);
    if let Err(error) = parser.load(&mut receiver, true) {
        receiver
            .diagnostics
            .push(yaml_parse_diagnostic(uri, &error));
    }
    receiver.diagnostics
}

struct YamlDiagnosticReceiver<'a> {
    uri: Option<&'a str>,
    diagnostics: Vec<Diagnostic>,
}

impl yaml_rust2::parser::MarkedEventReceiver for YamlDiagnosticReceiver<'_> {
    fn on_event(&mut self, ev: yaml_rust2::parser::Event, marker: yaml_rust2::scanner::Marker) {
        match ev {
            yaml_rust2::parser::Event::Scalar(_, _, _, Some(tag))
            | yaml_rust2::parser::Event::SequenceStart(_, Some(tag))
            | yaml_rust2::parser::Event::MappingStart(_, Some(tag)) => {
                if !is_safe_yaml_tag(&tag) {
                    self.diagnostics.push(generic_data_diagnostic(
                        self.uri,
                        u32::try_from(marker.line()).ok(),
                        u32::try_from(marker.col()).ok(),
                        "cem.yaml.unsafe_tag",
                        Severity::Error,
                        format!(
                            "YAML node uses unsupported explicit tag `{}`",
                            yaml_tag_display(&tag)
                        ),
                    ));
                }
            }
            _ => {}
        }
    }
}

fn is_safe_yaml_tag(tag: &yaml_rust2::parser::Tag) -> bool {
    let handle = tag.handle.trim();
    let suffix = tag.suffix.trim();
    if handle.is_empty() && suffix.is_empty() {
        return true;
    }

    match handle {
        "!" => suffix.is_empty(),
        "!!" | "tag:yaml.org,2002:" => is_safe_yaml_core_tag_name(suffix),
        _ => false,
    }
}

fn is_safe_yaml_core_tag_name(name: &str) -> bool {
    matches!(
        name,
        "binary"
            | "bool"
            | "float"
            | "int"
            | "map"
            | "merge"
            | "null"
            | "omap"
            | "pairs"
            | "seq"
            | "set"
            | "str"
            | "timestamp"
            | "value"
            | "yaml"
    )
}

fn yaml_tag_display(tag: &yaml_rust2::parser::Tag) -> String {
    format!("{}{}", tag.handle, tag.suffix)
}

fn generic_data_diagnostic(
    uri: Option<&str>,
    line: Option<u32>,
    column: Option<u32>,
    code: &str,
    severity: Severity,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        line,
        column,
        code: code.to_owned(),
        severity,
        message: message.into(),
        ..Diagnostic::default()
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
    pub formatted_cem_tree: Option<TransformTemplateEncodedArtifact>,
    pub colored_cem_tree: Option<TransformTemplateEncodedArtifact>,
    pub diagnostics: Vec<Diagnostic>,
}

const CEM_TREE_FORMAT_CEMT_ADAPTER_ID: &str = "cem-tree-format-cemt";
const CEM_TREE_COLOR_CEMT_ADAPTER_ID: &str = "cem-tree-color-cemt";
const CEMT_FORMATTER_COLORING_PIPELINE_PACKAGE_SOURCE_URI: &str =
    "schema-packages/cem-ml/v1/package.cem";
const CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND: &str = "formatter-helper";
const CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND: &str = "colorizer-helper";

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
    artifact_kind: "formatter",
    declaration_element: "{format-function",
    function_kind: TransformTemplateOutputFunctionKind::Format,
    function_name: "cem.format-tree",
    role: "formatter",
};

const CEM_TREE_COLOR_CEMT_STAGE_SPEC: CemTreeCemtOutputStageSpec = CemTreeCemtOutputStageSpec {
    adapter_id: CEM_TREE_COLOR_CEMT_ADAPTER_ID,
    artifact_kind: "colorizer",
    declaration_element: "{color-function",
    function_kind: TransformTemplateOutputFunctionKind::Color,
    function_name: "cem.color-tree",
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
    match artifact_kind.trim() {
        "formatter" | CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND => {
            Some(TransformTemplateOutputFunctionKind::Format)
        }
        "colorizer" | CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND => {
            Some(TransformTemplateOutputFunctionKind::Color)
        }
        _ => None,
    }
}

fn cemt_output_stage_helper_artifact_kind(
    function_kind: TransformTemplateOutputFunctionKind,
) -> Option<&'static str> {
    match function_kind {
        TransformTemplateOutputFunctionKind::Format => {
            Some(CEM_TREE_FORMATTER_HELPER_ARTIFACT_KIND)
        }
        TransformTemplateOutputFunctionKind::Color => Some(CEM_TREE_COLORIZER_HELPER_ARTIFACT_KIND),
        TransformTemplateOutputFunctionKind::Encoding => None,
    }
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
    if helper_name.as_str() == stage.function_name.as_str()
        || cemt_builtin_runtime_operation_name(&helper_name)
    {
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

fn cemt_builtin_runtime_operation_name(name: &str) -> bool {
    matches!(
        name,
        "cem.format-tree.nodes"
            | "cem.format-tree.inter-node-whitespace"
            | "cem.format-tree.block-children"
            | "cem.format-tree.content-boundary"
            | "cem.format-tree.format-nodes"
            | "cem.format-tree.envelope"
            | "cem.color-tree.apply"
    )
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
    let parsed_stage_function = module_options
        .output_functions
        .iter()
        .find(|function| {
            function.kind == stage.function_kind
                && function.name.as_str() == stage.function_name.as_str()
        })
        .cloned();
    let mut execution_binding = binding.clone();
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
    let (formatted_output, format_execution) = match execute_conversion_cem_tree_format_stage(
        environment,
        &format_binding,
        &rendered_value,
    ) {
        Ok(output) => output,
        Err(message) => {
            diagnostics.push(conversion_output_pipeline_diagnostic(
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
            ..ConversionOutputPipelineExecution::default()
        };
    }
    let formatted_cem_tree = Some(formatted_artifact.clone());

    let color_request = TransformTemplateEncodeBindingRequest::new(
        formatted_artifact.value.clone(),
        pipeline.cemt_target.clone(),
    )
    .with_subject_type("cem-tree")
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
                formatted_cem_tree: formatted_cem_tree.clone(),
                ..ConversionOutputPipelineExecution::default()
            };
        }
    };
    let (colored_output, color_execution) = match execute_conversion_cem_tree_color_stage(
        environment,
        &color_binding,
        &formatted_artifact.value,
    ) {
        Ok(output) => output,
        Err(message) => {
            diagnostics.push(conversion_output_pipeline_diagnostic(
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
    if let Err(error) = colored_artifact.validate_insertion(&pipeline.cemt_insertion_context) {
        let mut diagnostic = error.diagnostic(diagnostic_uri);
        diagnostic.node = diagnostic_node.map(str::to_owned);
        diagnostics.push(diagnostic);
        return ConversionOutputPipelineExecution {
            output: None,
            diagnostics,
            format_execution,
            color_execution,
            formatted_cem_tree,
            ..ConversionOutputPipelineExecution::default()
        };
    }
    let colored_cem_tree = Some(colored_artifact.clone());

    let evaluated = TransformTemplateEvaluatedEncodeExpression {
        expression: TransformTemplateEncodeExpression {
            owner: diagnostic_node.map(str::to_owned),
            expression: format!("{converter_id} output pipeline"),
            subject: "rendered-cem-tree".to_owned(),
            subject_type: Some("cem-tree".to_owned()),
            target: pipeline.cemt_target.clone(),
            options: pipeline.cemt_options.clone(),
        },
        subject: formatted_artifact.value.clone(),
        binding: color_binding,
        artifact: colored_artifact.clone(),
    };
    let composition = compose_transform_template_encoded_text_artifacts(
        &[evaluated],
        &pipeline.writer_insertion_context,
        diagnostic_uri,
    );
    diagnostics.extend(composition.diagnostics);
    match composition.artifact {
        Some(artifact) => ConversionOutputPipelineExecution {
            output: Some(artifact.value),
            source_map: artifact.source_map,
            output_spans: artifact.output_spans,
            format_execution,
            color_execution,
            formatted_cem_tree,
            colored_cem_tree,
            diagnostics,
        },
        None => ConversionOutputPipelineExecution {
            output: None,
            diagnostics,
            format_execution,
            color_execution,
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
        Some(TransformTemplateOutputProducedKind::CemTree),
    );
    context.formatter_profile = pipeline.cemt_options.formatter_profile.clone();
    context.mode = Some(pipeline.cemt_options.mode);
    context.canonical = Some(pipeline.cemt_options.canonical);
    context.source_map_policy = Some(pipeline.cemt_options.source_map_policy);
    context
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
            .unwrap_or("cem.format-tree"),
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
            .find(|function| function.kind == expected_kind && function.name == function_name)
        else {
            continue;
        };
        let key = (
            function.kind,
            function.name.clone(),
            function.profile.clone(),
        );
        if registered.insert(key) {
            registry.register(function.clone());
        }
    }
}

fn conversion_cem_tree_format_function_descriptor(
    profile: &str,
) -> TransformTemplateOutputFunctionDescriptor {
    TransformTemplateOutputFunctionDescriptor {
        kind: TransformTemplateOutputFunctionKind::Format,
        owner: Some("cem".to_owned()),
        name: "cem.format-tree".to_owned(),
        category: "cem-tree".to_owned(),
        subject: "cem-ast-node".to_owned(),
        produces: TransformTemplateOutputProducedKind::CemTree,
        content_type: CEM_ML_CONTENT_TYPE.to_owned(),
        schema: CEM_ML_SCHEMA_URI.to_owned(),
        canonical: true,
        streamable: true,
        visibility: TransformTemplateModuleVisibility::Public,
        implementation: TransformTemplateOutputFunctionImplementation::Cemt,
        profile: Some(profile.to_owned()),
        extends: None,
        capability: None,
        deterministic: true,
        trusted: false,
        lossy: false,
        fallback: None,
        params: Vec::new(),
        body_declared: false,
        body_expression: None,
    }
}

fn conversion_cem_tree_color_function_descriptor(
    profile: &str,
) -> TransformTemplateOutputFunctionDescriptor {
    TransformTemplateOutputFunctionDescriptor {
        kind: TransformTemplateOutputFunctionKind::Color,
        owner: Some("cem".to_owned()),
        name: "cem.color-tree".to_owned(),
        category: "cem-tree".to_owned(),
        subject: "cem-tree".to_owned(),
        produces: TransformTemplateOutputProducedKind::CemTree,
        content_type: CEM_ML_CONTENT_TYPE.to_owned(),
        schema: CEM_ML_SCHEMA_URI.to_owned(),
        canonical: false,
        streamable: true,
        visibility: TransformTemplateModuleVisibility::Public,
        implementation: TransformTemplateOutputFunctionImplementation::Cemt,
        profile: Some(profile.to_owned()),
        extends: None,
        capability: None,
        deterministic: true,
        trusted: false,
        lossy: false,
        fallback: None,
        params: Vec::new(),
        body_declared: false,
        body_expression: None,
    }
}

fn conversion_output_pipeline_diagnostic(
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
    message: String,
) -> Diagnostic {
    Diagnostic {
        uri: diagnostic_uri.map(str::to_owned),
        code: CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE.to_owned(),
        severity: Severity::Error,
        message: format!(
            "converter `{converter_id}` could not execute CEMT output pipeline: {message}"
        ),
        node: diagnostic_node.map(str::to_owned),
        ..Diagnostic::default()
    }
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
            .is_some_and(|profile| profile == "canonical"),
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
        formatter_profile: Some("cem.format-tree".to_owned()),
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
        formatter_profile: Some("canonical".to_owned()),
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
            "format-tree" | "cem.format-tree" => "cem.format-tree",
            _ => "cem.format-tree",
        })
        .unwrap_or("cem.format-tree")
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

    if matches!(selector, "none" | "plain") {
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
    let document = parse_cem_document(manifest_source);
    let package_id = package_manifest_package_id(package_id_hint, &document)?;
    let Some(package_node_id) = first_element_id_by_local_name(&document, "package") else {
        return Err(ConversionManifestError::MissingPackageElement);
    };

    let mut descriptors = Vec::new();
    for converter_id in element_child_ids_by_local_name(&document, package_node_id, "converter") {
        descriptors.push(conversion_descriptor_from_manifest_node(
            &document,
            converter_id,
            &package_id,
            base_path,
        )?);
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

    element_child_ids_by_local_name(&document, package_node_id, "artifact")
        .into_iter()
        .map(|artifact_node_id| {
            conversion_package_artifact_from_manifest_node(
                &document,
                artifact_node_id,
                &package_id,
                base_path,
                validate_embedded_artifact_contracts,
            )
        })
        .collect()
}

fn conversion_package_artifact_from_manifest_node(
    document: &CemDocument,
    node_id: AstNodeId,
    package_id: &str,
    base_path: &str,
    validate_embedded_artifact_contracts: bool,
) -> Result<ConversionPackageArtifactDescriptor, ConversionManifestError> {
    let attrs = collect_manifest_attrs(document, node_id);
    let kind = required_manifest_attr(&attrs, None, "kind")?.to_owned();
    let path = package_relative_path(base_path, required_manifest_attr(&attrs, None, "path")?);
    let content_type = optional_manifest_attr(&attrs, "content-type").map(content_type_essence);
    let schema = optional_manifest_attr(&attrs, "schema").map(str::to_owned);
    let target_content_type =
        optional_manifest_attr(&attrs, "target-content-type").map(content_type_essence);
    let target_schema = optional_manifest_attr(&attrs, "target-schema").map(str::to_owned);
    let target_category = optional_manifest_attr(&attrs, "target-category").map(str::to_owned);
    let generated = parse_manifest_bool("artifact", &attrs, "generated")?.unwrap_or(false);
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
    if validate_embedded_artifact_contracts {
        validate_conversion_package_artifact_cemt_contract(&artifact)?;
    }
    Ok(artifact)
}

fn validate_conversion_package_artifact_cemt_contract(
    artifact: &ConversionPackageArtifactDescriptor,
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
    Ok(())
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

fn conversion_descriptor_from_manifest_node(
    document: &CemDocument,
    node_id: AstNodeId,
    package_id: &str,
    base_path: &str,
) -> Result<ConversionDescriptor, ConversionManifestError> {
    let attrs = collect_manifest_attrs(document, node_id);
    let id = required_manifest_attr(&attrs, None, "id")?.to_owned();
    let implementation = parse_manifest_implementation(
        &id,
        required_manifest_attr(&attrs, Some(&id), "implementation")?,
    )?;
    let from = manifest_endpoint(document, node_id, &id, "from")?;
    let to = manifest_endpoint(document, node_id, &id, "to")?;
    let readiness = attrs
        .get("readiness")
        .map(|value| parse_manifest_readiness(&id, value))
        .transpose()?
        .unwrap_or(ConversionReadiness::Ready);
    let streamable = parse_manifest_bool(&id, &attrs, "streamable")?.unwrap_or(false);
    let explicit_only = parse_manifest_bool(&id, &attrs, "explicit-only")?.unwrap_or(false);
    let implicit = parse_manifest_bool(&id, &attrs, "implicit")?.unwrap_or(!explicit_only);
    let cost = parse_manifest_cost(&id, &attrs)?.unwrap_or(100);
    let output_contract = ConversionOutputContractDescriptor {
        output_syntax: optional_manifest_attr(&attrs, "output-syntax")
            .map(|value| parse_manifest_output_syntax(&id, value))
            .transpose()?,
        encoding_category: optional_manifest_attr(&attrs, "encoding-category").map(str::to_owned),
        formatter_profile: optional_manifest_attr(&attrs, "formatter-profile").map(str::to_owned),
        color_profile: optional_manifest_attr(&attrs, "color-profile").map(str::to_owned),
        parity: optional_manifest_attr(&attrs, "parity")
            .map(|value| parse_manifest_parity_mode(&id, value))
            .transpose()?,
    };
    let parity_fixtures = manifest_parity_fixtures(document, node_id, &id, base_path)?;

    let template = match implementation {
        ConversionImplementation::Cemt => {
            let template_path = required_manifest_attr(&attrs, Some(&id), "template")?;
            let template_content_type =
                required_manifest_attr(&attrs, Some(&id), "template-content-type")?;
            Some(ConversionTemplateDescriptor {
                path: package_relative_path(base_path, template_path),
                content_type: content_type_essence(template_content_type),
                schema: optional_manifest_attr(&attrs, "template-schema").map(str::to_owned),
                entrypoint: optional_manifest_attr(&attrs, "template-entrypoint")
                    .map(str::to_owned),
            })
        }
        ConversionImplementation::Rust => None,
    };

    let rust_symbol = optional_manifest_attr(&attrs, "rust-symbol").map(str::to_owned);
    let (rust_symbol, rust_fallback) = match implementation {
        ConversionImplementation::Cemt => (
            None,
            rust_symbol
                .map(|rust_symbol| {
                    let reason = required_manifest_attr(&attrs, Some(&id), "fallback-reason")?;
                    Ok(ConversionRustFallbackDescriptor {
                        rust_symbol,
                        reason: reason.to_owned(),
                    })
                })
                .transpose()?,
        ),
        ConversionImplementation::Rust => (
            Some(
                rust_symbol.ok_or_else(|| ConversionManifestError::MissingAttribute {
                    converter_id: Some(id.clone()),
                    attribute: "rust-symbol",
                })?,
            ),
            None,
        ),
    };

    Ok(ConversionDescriptor {
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
    converter_id: &str,
    base_path: &str,
) -> Result<Vec<ConversionParityFixtureDescriptor>, ConversionManifestError> {
    element_child_ids_by_local_name(document, converter_node_id, "parity-fixture")
        .into_iter()
        .map(|fixture_node_id| {
            let attrs = collect_manifest_attrs(document, fixture_node_id);
            let id = required_manifest_attr(&attrs, Some(converter_id), "id")?.to_owned();
            let path = package_relative_path(
                base_path,
                required_manifest_attr(&attrs, Some(converter_id), "path")?,
            );
            let content_type =
                optional_manifest_attr(&attrs, "content-type").map(content_type_essence);
            let schema = optional_manifest_attr(&attrs, "schema").map(str::to_owned);
            let expected_diagnostic_codes = optional_manifest_attr(&attrs, "expected-diagnostics")
                .map(|value| {
                    value
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            Ok(ConversionParityFixtureDescriptor {
                id,
                path,
                content_type,
                schema,
                expected_diagnostic_codes,
            })
        })
        .collect()
}

fn manifest_endpoint(
    document: &CemDocument,
    converter_node_id: AstNodeId,
    converter_id: &str,
    endpoint_name: &'static str,
) -> Result<ConversionEndpoint, ConversionManifestError> {
    let endpoint_id = element_child_ids_by_local_name(document, converter_node_id, endpoint_name)
        .into_iter()
        .next()
        .ok_or_else(|| ConversionManifestError::MissingEndpoint {
            converter_id: converter_id.to_owned(),
            endpoint: endpoint_name,
        })?;
    let attrs = collect_manifest_attrs(document, endpoint_id);
    let content_type = required_manifest_attr(&attrs, Some(converter_id), "content-type")?;
    Ok(match optional_manifest_attr(&attrs, "schema") {
        Some(schema) => ConversionEndpoint::with_schema(content_type, schema),
        None => ConversionEndpoint::new(content_type),
    })
}

fn parse_manifest_implementation(
    converter_id: &str,
    value: &str,
) -> Result<ConversionImplementation, ConversionManifestError> {
    match value.trim() {
        "cemt" => Ok(ConversionImplementation::Cemt),
        "rust" => Ok(ConversionImplementation::Rust),
        implementation => Err(ConversionManifestError::UnknownImplementation {
            converter_id: converter_id.to_owned(),
            implementation: implementation.to_owned(),
        }),
    }
}

fn parse_manifest_readiness(
    converter_id: &str,
    value: &str,
) -> Result<ConversionReadiness, ConversionManifestError> {
    match value.trim() {
        "ready" => Ok(ConversionReadiness::Ready),
        "planned" => Ok(ConversionReadiness::Planned),
        readiness => Err(ConversionManifestError::UnknownReadiness {
            converter_id: converter_id.to_owned(),
            readiness: readiness.to_owned(),
        }),
    }
}

fn parse_manifest_output_syntax(
    converter_id: &str,
    value: &str,
) -> Result<ConversionOutputSyntax, ConversionManifestError> {
    match value.trim() {
        "html" => Ok(ConversionOutputSyntax::Html),
        "xml" => Ok(ConversionOutputSyntax::Xml),
        "json" => Ok(ConversionOutputSyntax::Json),
        "yaml" => Ok(ConversionOutputSyntax::Yaml),
        "csv" => Ok(ConversionOutputSyntax::Csv),
        "css" => Ok(ConversionOutputSyntax::Css),
        "markdown" => Ok(ConversionOutputSyntax::Markdown),
        "cemt" => Ok(ConversionOutputSyntax::Cemt),
        "text" => Ok(ConversionOutputSyntax::Text),
        "binary" => Ok(ConversionOutputSyntax::Binary),
        "opaque" => Ok(ConversionOutputSyntax::Opaque),
        output_syntax => Err(ConversionManifestError::UnknownOutputSyntax {
            converter_id: converter_id.to_owned(),
            output_syntax: output_syntax.to_owned(),
        }),
    }
}

fn parse_manifest_parity_mode(
    converter_id: &str,
    value: &str,
) -> Result<ConversionParityMode, ConversionManifestError> {
    match value.trim() {
        "byte-exact" => Ok(ConversionParityMode::ByteExact),
        "token-equivalent" => Ok(ConversionParityMode::TokenEquivalent),
        "parse-equivalent" => Ok(ConversionParityMode::ParseEquivalent),
        "diagnostic-equivalent" => Ok(ConversionParityMode::DiagnosticEquivalent),
        parity => Err(ConversionManifestError::UnknownParityMode {
            converter_id: converter_id.to_owned(),
            parity: parity.to_owned(),
        }),
    }
}

fn parse_manifest_bool(
    converter_id: &str,
    attrs: &BTreeMap<String, String>,
    attribute: &'static str,
) -> Result<Option<bool>, ConversionManifestError> {
    let Some(value) = attrs.get(attribute).map(String::as_str).map(str::trim) else {
        return Ok(None);
    };
    match value {
        "" | "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        _ => Err(ConversionManifestError::InvalidBoolean {
            converter_id: converter_id.to_owned(),
            attribute,
            value: value.to_owned(),
        }),
    }
}

fn parse_manifest_cost(
    converter_id: &str,
    attrs: &BTreeMap<String, String>,
) -> Result<Option<u32>, ConversionManifestError> {
    let Some(value) = optional_manifest_attr(attrs, "cost") else {
        return Ok(None);
    };
    value
        .parse::<u32>()
        .map(Some)
        .map_err(|_| ConversionManifestError::InvalidCost {
            converter_id: converter_id.to_owned(),
            value: value.to_owned(),
        })
}

fn required_manifest_attr<'a>(
    attrs: &'a BTreeMap<String, String>,
    converter_id: Option<&str>,
    attribute: &'static str,
) -> Result<&'a str, ConversionManifestError> {
    optional_manifest_attr(attrs, attribute).ok_or_else(|| {
        ConversionManifestError::MissingAttribute {
            converter_id: converter_id.map(str::to_owned),
            attribute,
        }
    })
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
    use crate::engine::TransformTemplateKind;
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
            Some("canonical")
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
    fn package_manifest_requires_cemt_fallback_reason() {
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
        @rust-symbol="HtmlExportConverter"
        @output-syntax="html"
        @encoding-category="html-document"
        @parity="parse-equivalent" |
        {from @content-type="application/vnd.cem.dom+cem-bin" @schema="https://cem.dev/ns/projection/dom/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}"#,
        );

        let error = conversion_descriptors_from_schema_package(&package)
            .expect_err("CEMT fallback reason is required when rust-symbol is declared");

        assert_eq!(
            error,
            ConversionManifestError::MissingAttribute {
                converter_id: Some("dom-to-html-cemt".to_owned()),
                attribute: "fallback-reason",
            }
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
            Some("canonical")
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
            Some("cem.format-tree")
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
            Some("cem.format-tree")
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
                "formatterProfile": "cem.format-tree",
                "formatNodes": [{
                    "kind": "format-marker",
                    "name": "cem.format-tree",
                    "formatterRole": "formatter.boundary",
                    "formatterProfile": "cem.format-tree"
                }, {
                    "kind": "format-decision",
                    "name": "line-ending",
                    "formatterRole": "formatter.line-ending",
                    "value": "lf",
                    "formatterProfile": "cem.format-tree"
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
        assert_eq!(formatted.value["formatterProfile"], "cem.format-tree");
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
            formatter_profile: Some("cem.format-tree".to_owned()),
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
            formatter_profile: Some("cem.format-tree".to_owned()),
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
            formatter_profile: Some("cem.format-tree".to_owned()),
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
        let formatter_profile = "cem.format-tree";
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
            Some(formatter_profile)
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
        for source in [formatter_source.source, colorizer_source.source] {
            assert!(source.contains(r#"@content-type="application/cem""#));
            assert!(source.contains(r#"@schema="https://cem.dev/ns/cem-ml/1""#));
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
        let stage = cem_tree_format_cemt_stage(&target, Some("cem.format-tree"))
            .expect("CEM tree formatter stage");
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
            Some("json")
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
            .is_some_and(|body| body.contains("call(cem.format-tree.nodes")));
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
            .is_some_and(|body| body.contains("call(cem.format-tree.envelope")));
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
        assert!(body.contains("appendColorNode("));
        assert!(body.contains("applyEdits("));
        assert!(body.contains("call(cem.color-tree.apply"));
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
            stage_profile: Some("cem.format-tree".to_owned()),
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
                formatterProfile: "cem.format-tree",
                formatNodes: [
                    {
                        kind: "format-marker",
                        name: "cem.format-tree",
                        formatterRole: "formatter.boundary",
                        formatterProfile: "cem.format-tree"
                    },
                    {
                        kind: "format-decision",
                        name: "layout",
                        formatterRole: "formatter.layout",
                        value: "direct-cemt",
                        formatterProfile: "cem.format-tree"
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
        assert_eq!(formatted["formatterProfile"], "cem.format-tree");
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
            stage_profile: Some("cem.format-tree".to_owned()),
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
            stage_profile: Some("cem.format-tree".to_owned()),
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
        {param @name="subject" @type="json" @required=true}
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
        {param @name="subject" @type="json" @required=true}
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
            "formatterProfile": "cem.format-tree",
            "formatNodes": [
                {
                    "kind": "format-marker",
                    "name": "cem.format-tree",
                    "formatterRole": "formatter.boundary",
                    "formatterProfile": "cem.format-tree"
                },
                {
                    "kind": "format-decision",
                    "name": "layout",
                    "formatterRole": "formatter.layout",
                    "value": "direct-cemt",
                    "formatterProfile": "cem.format-tree"
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
