//! Transform template adapter registry.
//!
//! Template documents are versioned independently from the base CEM-ML
//! document/AST language. This registry keeps template content-type and schema
//! dispatch pluggable so CEM-native template iterations can ship as built-in
//! adapters or be installed by hosts at runtime.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{
    FormatIdentity, TemplateInput, TransformDiagnosticOrigin, TransformExecutionPolicy,
    TransformTemplateEntrypoint, TransformTemplateKind,
};
use crate::events::cem::CemEventNormalizer;
use crate::interpreter::OutputSpan;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::run_config::ScopeConfig;
use crate::schema::registry::{
    CEM_NATIVE_TEMPLATE_CONTENT_TYPE, CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI,
    XSLT_SCHEMA_URI,
};
use crate::source::{BytesSource, SourceId};
use crate::source_map::SourceMapStack;
use crate::tokenizer::cem::CemTokenizer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

pub const CEM_NATIVE_TEMPLATE_SCHEMA_URI: &str = "https://cem.dev/ns/template/cem-native/1";
pub const CEM_NATIVE_TEMPLATE_NAMESPACE_URI: &str = CEM_NATIVE_TEMPLATE_SCHEMA_URI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformTemplateNativeElementSchema {
    pub local_name: &'static str,
    pub required_attributes: &'static [&'static str],
    pub optional_attributes: &'static [&'static str],
    pub child_elements: &'static [&'static str],
}

pub const CEM_NATIVE_TEMPLATE_SCHEMA_ELEMENTS: &[TransformTemplateNativeElementSchema] = &[
    TransformTemplateNativeElementSchema {
        local_name: "module",
        required_attributes: &[],
        optional_attributes: &["version"],
        child_elements: &["import", "param", "template", "body"],
    },
    TransformTemplateNativeElementSchema {
        local_name: "import",
        required_attributes: &["as", "src"],
        optional_attributes: &["content-type", "contentType", "schema"],
        child_elements: &[],
    },
    TransformTemplateNativeElementSchema {
        local_name: "param",
        required_attributes: &["name"],
        optional_attributes: &[
            "default",
            "default-expr",
            "defaultExpr",
            "nullable",
            "required",
            "type",
            "visibility",
        ],
        child_elements: &[],
    },
    TransformTemplateNativeElementSchema {
        local_name: "template",
        required_attributes: &["name"],
        optional_attributes: &["visibility"],
        child_elements: &["param", "body"],
    },
    TransformTemplateNativeElementSchema {
        local_name: "body",
        required_attributes: &[],
        optional_attributes: &[],
        child_elements: &["*"],
    },
    TransformTemplateNativeElementSchema {
        local_name: "call",
        required_attributes: &["template"],
        optional_attributes: &["from", "with:*"],
        child_elements: &[],
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformTemplateAdapterSelection {
    pub adapter_id: &'static str,
    pub kind: TransformTemplateKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformTemplateAdapterResolution {
    Matched(TransformTemplateAdapterSelection),
    Ambiguous(Vec<&'static str>),
    Unsupported,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateAdapterCapability {
    #[default]
    SelectorOnly,
    Executable,
}

#[derive(Clone)]
pub enum TransformTemplateAdapterLookup {
    Matched(Arc<dyn TransformTemplateAdapter>),
    Ambiguous(Vec<&'static str>),
    Unsupported,
}

impl fmt::Debug for TransformTemplateAdapterLookup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Matched(adapter) => f.debug_tuple("Matched").field(&adapter.id()).finish(),
            Self::Ambiguous(ids) => f.debug_tuple("Ambiguous").field(ids).finish(),
            Self::Unsupported => f.write_str("Unsupported"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransformTemplateCompileRequest<'a> {
    pub template: &'a TemplateInput,
    pub entrypoint: &'a TransformTemplateEntrypoint,
    pub params: &'a BTreeMap<String, Value>,
    pub data_bindings: &'a [String],
    pub module_options: TransformTemplateModuleOptions,
    pub module_preflight: TransformTemplateModulePreflight,
    pub execution_policy: TransformExecutionPolicy,
}

pub const TRANSFORM_TEMPLATE_ENTRYPOINT_NOT_PUBLIC_CODE: &str =
    "cem.transform_template.entrypoint_not_public";
pub const TRANSFORM_TEMPLATE_PARAM_UNKNOWN_CODE: &str = "cem.transform_template.param_unknown";
pub const TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE: &str = "cem.transform_template.param_required";
pub const TRANSFORM_TEMPLATE_PARAM_DUPLICATE_ALIAS_CODE: &str =
    "cem.transform_template.param_duplicate_alias";
pub const TRANSFORM_TEMPLATE_PARAM_TYPE_CODE: &str = "cem.transform_template.param_type";
pub const TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE: &str = "cem.transform_template.call_unknown";
pub const TRANSFORM_TEMPLATE_IMPORT_CYCLE_CODE: &str = "cem.transform_template.import_cycle";
pub const TRANSFORM_TEMPLATE_IMPORT_DEPTH_CODE: &str = "cem.transform_template.import_depth";
pub const TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE: &str = "cem.transform_template.recursion_limit";
pub const TRANSFORM_TEMPLATE_INCLUDE_RESERVED_CODE: &str =
    "cem.transform_template.include_reserved";
pub const TRANSFORM_TEMPLATE_PARAM_DEFAULT_EXPR_RESERVED_CODE: &str =
    "cem.transform_template.param_default_expr_reserved";
pub const TRANSFORM_TEMPLATE_IMPORT_ALIAS_DUPLICATE_CODE: &str =
    "cem.transform_template.import_alias_duplicate";
pub const TRANSFORM_TEMPLATE_DECLARATION_UNSUPPORTED_CODE: &str =
    "cem.transform_template.declaration_unsupported";
pub const TRANSFORM_TEMPLATE_DECLARATION_REQUIRED_CODE: &str =
    "cem.transform_template.declaration_required";
pub const TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE: &str =
    "cem.transform_template.declaration_duplicate";
pub const TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE: &str =
    "cem.transform_template.declaration_invalid";
pub const TRANSFORM_TEMPLATE_OUTPUT_FUNCTION_UNKNOWN_CODE: &str =
    "cem.transform_template.output_function_unknown";
pub const TRANSFORM_TEMPLATE_OUTPUT_FUNCTION_AMBIGUOUS_CODE: &str =
    "cem.transform_template.output_function_ambiguous";
pub const TRANSFORM_TEMPLATE_OUTPUT_FUNCTION_CAPABILITY_MISSING_CODE: &str =
    "cem.transform_template.output_function_capability_missing";

#[derive(Debug, Clone)]
pub struct TransformTemplateModuleParseRequest {
    pub template: TemplateInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleParseResponse {
    pub module_options: TransformTemplateModuleOptions,
    pub diagnostics: Vec<Diagnostic>,
    pub module_declared: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateModuleDependencyKind {
    #[default]
    Import,
    IncludeReserved,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateModuleVisibility {
    #[default]
    Private,
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleImport {
    pub alias: String,
    pub uri: String,
    #[serde(default)]
    pub identity: Option<FormatIdentity>,
    #[serde(default)]
    pub kind: TransformTemplateModuleDependencyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleEntrypointDeclaration {
    pub name: String,
    #[serde(default)]
    pub visibility: TransformTemplateModuleVisibility,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleParamDeclaration {
    pub name: String,
    #[serde(
        default,
        rename = "type",
        skip_serializing_if = "TransformTemplateModuleParamType::is_any"
    )]
    pub value_type: TransformTemplateModuleParamType,
    #[serde(default, skip_serializing_if = "is_false")]
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub visibility: TransformTemplateModuleVisibility,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateOutputFunctionKind {
    #[default]
    Encoding,
    Format,
    Color,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateOutputProducedKind {
    #[default]
    Text,
    Bytes,
    Tokens,
    Chunks,
    Diagnostics,
}

impl TransformTemplateOutputProducedKind {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "text" => Some(Self::Text),
            "bytes" => Some(Self::Bytes),
            "tokens" => Some(Self::Tokens),
            "chunks" => Some(Self::Chunks),
            "diagnostics" => Some(Self::Diagnostics),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateOutputFunctionImplementation {
    #[default]
    Cemt,
    Native,
    External,
}

impl TransformTemplateOutputFunctionImplementation {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "cemt" => Some(Self::Cemt),
            "native" => Some(Self::Native),
            "external" => Some(Self::External),
            _ => None,
        }
    }

    fn requires_capability(self) -> bool {
        matches!(self, Self::Native | Self::External)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateOutputFunctionDescriptor {
    pub kind: TransformTemplateOutputFunctionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub name: String,
    pub category: String,
    pub subject: String,
    pub produces: TransformTemplateOutputProducedKind,
    pub content_type: String,
    pub schema: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub canonical: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub streamable: bool,
    #[serde(default)]
    pub visibility: TransformTemplateModuleVisibility,
    #[serde(default)]
    pub implementation: TransformTemplateOutputFunctionImplementation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub deterministic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub trusted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<TransformTemplateModuleParamDeclaration>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub body_declared: bool,
}

impl TransformTemplateOutputFunctionDescriptor {
    fn key(&self) -> TransformTemplateOutputFunctionKey {
        TransformTemplateOutputFunctionKey {
            kind: self.kind,
            owner: self.owner.clone(),
            name: self.name.clone(),
            content_type: content_type_essence(&self.content_type),
            schema: self.schema.clone(),
            category: self.category.clone(),
            subject: self.subject.clone(),
            profile: self.profile.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransformTemplateOutputFunctionQuery {
    pub kind: Option<TransformTemplateOutputFunctionKind>,
    pub owner: Option<String>,
    pub name: Option<String>,
    pub content_type: Option<String>,
    pub schema: Option<String>,
    pub category: Option<String>,
    pub subject: Option<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TransformTemplateOutputFunctionKey {
    kind: TransformTemplateOutputFunctionKind,
    owner: Option<String>,
    name: String,
    content_type: String,
    schema: String,
    category: String,
    subject: String,
    profile: Option<String>,
}

fn output_function_matches(
    function: &TransformTemplateOutputFunctionDescriptor,
    query: &TransformTemplateOutputFunctionQuery,
) -> bool {
    query.kind.is_none_or(|kind| function.kind == kind)
        && query
            .owner
            .as_deref()
            .is_none_or(|owner| function.owner.as_deref() == Some(owner))
        && query
            .name
            .as_deref()
            .is_none_or(|name| function.name == name)
        && query.content_type.as_deref().is_none_or(|content_type| {
            content_type_essence(&function.content_type) == content_type_essence(content_type)
        })
        && query
            .schema
            .as_deref()
            .is_none_or(|schema| function.schema == schema)
        && query
            .category
            .as_deref()
            .is_none_or(|category| function.category == category)
        && query
            .subject
            .as_deref()
            .is_none_or(|subject| function.subject == subject)
        && query
            .profile
            .as_deref()
            .is_none_or(|profile| function.profile.as_deref() == Some(profile))
}

impl TransformTemplateOutputFunctionQuery {
    pub fn for_identity(
        kind: TransformTemplateOutputFunctionKind,
        content_type: impl Into<String>,
        schema: impl Into<String>,
        category: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            kind: Some(kind),
            content_type: Some(content_type.into()),
            schema: Some(schema.into()),
            category: Some(category.into()),
            subject: Some(subject.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformTemplateOutputFunctionResolutionError {
    Unknown,
    Ambiguous {
        function_names: Vec<String>,
    },
    MissingCapability {
        function_name: String,
        capability: String,
    },
}

impl TransformTemplateOutputFunctionResolutionError {
    pub fn diagnostic(&self, uri: Option<&str>) -> Diagnostic {
        match self {
            Self::Unknown => Diagnostic {
                uri: uri.map(str::to_owned),
                code: TRANSFORM_TEMPLATE_OUTPUT_FUNCTION_UNKNOWN_CODE.to_owned(),
                severity: Severity::Error,
                message: "no CEMT output function matched the requested identity".to_owned(),
                ..Diagnostic::default()
            },
            Self::Ambiguous { function_names } => Diagnostic {
                uri: uri.map(str::to_owned),
                code: TRANSFORM_TEMPLATE_OUTPUT_FUNCTION_AMBIGUOUS_CODE.to_owned(),
                severity: Severity::Error,
                message: format!(
                    "CEMT output function resolution is ambiguous: {}",
                    function_names.join(", ")
                ),
                ..Diagnostic::default()
            },
            Self::MissingCapability {
                function_name,
                capability,
            } => Diagnostic {
                uri: uri.map(str::to_owned),
                code: TRANSFORM_TEMPLATE_OUTPUT_FUNCTION_CAPABILITY_MISSING_CODE.to_owned(),
                severity: Severity::Error,
                message: format!(
                    "CEMT output function `{function_name}` requires unavailable capability `{capability}`"
                ),
                ..Diagnostic::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransformTemplateOutputFunctionRegistry {
    functions: Vec<TransformTemplateOutputFunctionDescriptor>,
}

impl TransformTemplateOutputFunctionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_module_options(options: &TransformTemplateModuleOptions) -> Self {
        Self {
            functions: options.output_functions.clone(),
        }
    }

    pub fn register(&mut self, function: TransformTemplateOutputFunctionDescriptor) {
        self.functions.push(function);
    }

    pub fn functions(&self) -> &[TransformTemplateOutputFunctionDescriptor] {
        &self.functions
    }

    pub fn resolve<'a>(
        &'a self,
        query: &TransformTemplateOutputFunctionQuery,
        host_capabilities: &BTreeSet<String>,
    ) -> Result<
        &'a TransformTemplateOutputFunctionDescriptor,
        TransformTemplateOutputFunctionResolutionError,
    > {
        let matches = self
            .functions
            .iter()
            .filter(|function| output_function_matches(function, query))
            .collect::<Vec<_>>();

        let function = match matches.as_slice() {
            [] => return Err(TransformTemplateOutputFunctionResolutionError::Unknown),
            [function] => *function,
            many => {
                return Err(TransformTemplateOutputFunctionResolutionError::Ambiguous {
                    function_names: many.iter().map(|function| function.name.clone()).collect(),
                })
            }
        };

        if function.implementation.requires_capability() {
            let Some(capability) = function.capability.as_deref() else {
                return Err(
                    TransformTemplateOutputFunctionResolutionError::MissingCapability {
                        function_name: function.name.clone(),
                        capability: "<missing>".to_owned(),
                    },
                );
            };
            if !host_capabilities.contains(capability) {
                return Err(
                    TransformTemplateOutputFunctionResolutionError::MissingCapability {
                        function_name: function.name.clone(),
                        capability: capability.to_owned(),
                    },
                );
            }
        }

        Ok(function)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateModuleParamType {
    #[default]
    Any,
    String,
    Boolean,
    Number,
    Integer,
    Array,
    Object,
    Json,
}

impl TransformTemplateModuleParamType {
    pub fn is_any(value: &Self) -> bool {
        *value == Self::Any
    }

    pub fn accepts(self, value: &Value, nullable: bool) -> bool {
        if value.is_null() {
            return nullable;
        }
        match self {
            Self::Any | Self::Json => true,
            Self::String => value.is_string(),
            Self::Boolean => value.is_boolean(),
            Self::Number => value.is_number(),
            Self::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }

    pub fn as_contract_name(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::String => "string",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Array => "array",
            Self::Object => "object",
            Self::Json => "json",
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleCallSite {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_entrypoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub template: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleLimits {
    pub max_import_depth: u32,
    pub max_recursion_depth: u32,
}

impl Default for TransformTemplateModuleLimits {
    fn default() -> Self {
        Self {
            max_import_depth: 32,
            max_recursion_depth: 64,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleOptions {
    #[serde(default)]
    pub imports: Vec<TransformTemplateModuleImport>,
    #[serde(default)]
    pub entrypoints: Vec<TransformTemplateModuleEntrypointDeclaration>,
    #[serde(default)]
    pub params: Vec<TransformTemplateModuleParamDeclaration>,
    #[serde(default)]
    pub calls: Vec<TransformTemplateModuleCallSite>,
    #[serde(default)]
    pub output_functions: Vec<TransformTemplateOutputFunctionDescriptor>,
    #[serde(default)]
    pub limits: TransformTemplateModuleLimits,
}

pub fn parse_cem_native_template_module_options(
    request: TransformTemplateModuleParseRequest,
) -> TransformTemplateModuleParseResponse {
    let explicit_template_schema = template_has_native_module_schema(&request.template);
    let explicit_transform_schema = template_has_transform_module_schema(&request.template);
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), request.template.bytes));
    let tokenizer_diagnostics = tokenizer.take_diagnostics();
    let normalizer = CemEventNormalizer::new(tokenizer);
    let document = CemAstBuilder::new(normalizer).build();
    let mut parser = NativeTemplateModuleLowerer {
        document: &document,
        template_uri: request.template.uri.as_str(),
        options: TransformTemplateModuleOptions::default(),
        diagnostics: Vec::new(),
        module_count: 0,
        saw_doc_directive: false,
        explicit_template_schema,
        explicit_transform_schema,
    };
    parser.lower_document();
    parser.validate_declarations();

    let module_declared = parser.module_count > 0;
    let mut diagnostics = parser.diagnostics;
    if module_declared || explicit_template_schema {
        diagnostics.extend(tokenizer_diagnostics);
        diagnostics.extend(document.diagnostics.clone());
    }

    TransformTemplateModuleParseResponse {
        module_options: if module_declared {
            parser.options
        } else {
            TransformTemplateModuleOptions::default()
        },
        diagnostics,
        module_declared,
    }
}

fn template_has_native_module_schema(template: &TemplateInput) -> bool {
    template.identity.as_ref().is_some_and(|identity| {
        identity.schema.as_deref().map(str::trim) == Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI)
            || identity.default_namespace.as_deref().map(str::trim)
                == Some(CEM_NATIVE_TEMPLATE_NAMESPACE_URI)
    })
}

fn template_has_transform_module_schema(template: &TemplateInput) -> bool {
    template.identity.as_ref().is_some_and(|identity| {
        identity
            .content_type
            .as_deref()
            .is_some_and(|content_type| {
                content_type_essence(content_type) == CEM_TRANSFORM_CONTENT_TYPE
            })
            || identity.schema.as_deref().map(str::trim) == Some(CEM_TRANSFORM_SCHEMA_URI)
            || identity.default_namespace.as_deref().map(str::trim)
                == Some(CEM_TRANSFORM_SCHEMA_URI)
    })
}

struct NativeTemplateModuleLowerer<'a> {
    document: &'a CemDocument,
    template_uri: &'a str,
    options: TransformTemplateModuleOptions,
    diagnostics: Vec<Diagnostic>,
    module_count: usize,
    saw_doc_directive: bool,
    explicit_template_schema: bool,
    explicit_transform_schema: bool,
}

impl NativeTemplateModuleLowerer<'_> {
    fn lower_document(&mut self) {
        let Some(CemAstNode::Document { root_children, .. }) = self.document.root() else {
            self.push_diag(
                TRANSFORM_TEMPLATE_DECLARATION_REQUIRED_CODE,
                "CEM-native template has no document root",
            );
            return;
        };

        for child in root_children {
            let Some(name) = template_element_name(self.document, *child) else {
                continue;
            };
            match name {
                "@doc" => self.saw_doc_directive = true,
                directive if directive.starts_with('@') => {}
                "module" => {
                    self.module_count += 1;
                    self.lower_module(*child);
                }
                other if self.explicit_template_schema || self.explicit_transform_schema => self
                    .push_diag(
                        TRANSFORM_TEMPLATE_DECLARATION_UNSUPPORTED_CODE,
                        format!("top-level `{other}` is not valid in CEM template module schema"),
                    ),
                _ => {}
            }
        }

        if (self.explicit_template_schema || self.explicit_transform_schema)
            && self.module_count == 0
        {
            self.push_diag(
                TRANSFORM_TEMPLATE_DECLARATION_REQUIRED_CODE,
                "CEM template schema requires one top-level `module` node",
            );
        } else if self.module_count > 1 {
            self.push_diag(
                TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE,
                "CEM-native template schema allows only one top-level `module` node",
            );
        }
    }

    fn lower_module(&mut self, module_id: AstNodeId) {
        let Some(CemAstNode::Element { children, .. }) = self.document.get(module_id) else {
            return;
        };

        for child in children {
            let Some(name) = template_element_name(self.document, *child) else {
                continue;
            };
            match name {
                "import" => self.lower_import(*child),
                "param" => self.lower_param(*child, None),
                "template" => self.lower_template(*child),
                "body" => self.collect_body_calls(*child, None),
                "encoding-function" if self.explicit_transform_schema => self
                    .lower_output_function(*child, TransformTemplateOutputFunctionKind::Encoding),
                "format-function" if self.explicit_transform_schema => {
                    self.lower_output_function(*child, TransformTemplateOutputFunctionKind::Format)
                }
                "color-function" if self.explicit_transform_schema => {
                    self.lower_output_function(*child, TransformTemplateOutputFunctionKind::Color)
                }
                "include" => self.push_diag(
                    TRANSFORM_TEMPLATE_INCLUDE_RESERVED_CODE,
                    "`include` is reserved in CEM-native template modules; use `import`",
                ),
                other => self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_UNSUPPORTED_CODE,
                    format!("`{other}` is not valid inside CEM-native template `module`"),
                ),
            }
        }
    }

    fn lower_import(&mut self, import_id: AstNodeId) {
        let attrs = template_collect_attrs(self.document, import_id);
        let Some(alias) = required_attr(&attrs, "as") else {
            self.push_missing_attr("import", "as");
            return;
        };
        let Some(uri) = required_attr(&attrs, "src") else {
            self.push_missing_attr("import", "src");
            return;
        };
        let identity = import_identity_from_attrs(&attrs);
        self.options.imports.push(TransformTemplateModuleImport {
            alias,
            uri,
            identity,
            kind: TransformTemplateModuleDependencyKind::Import,
        });
        self.reject_decl_children(import_id, "import");
    }

    fn lower_param(&mut self, param_id: AstNodeId, owner_entrypoint: Option<&str>) {
        let Some(mut declaration) = self.parse_param_declaration(param_id) else {
            return;
        };
        if let Some(entrypoint) = owner_entrypoint {
            declaration.name = format!("{entrypoint}.{}", declaration.name);
        }
        self.options.params.push(declaration);
        self.reject_decl_children(param_id, "param");
    }

    fn parse_param_declaration(
        &mut self,
        param_id: AstNodeId,
    ) -> Option<TransformTemplateModuleParamDeclaration> {
        let attrs = template_collect_attrs(self.document, param_id);
        let Some(name) = required_attr(&attrs, "name") else {
            self.push_missing_attr("param", "name");
            return None;
        };
        let visibility = self.parse_visibility(attr_value(&attrs, "", "visibility").as_deref());
        let value_type = self.parse_param_type(attr_value(&attrs, "", "type").as_deref());
        let nullable = parse_bool_attr(attr_value(&attrs, "", "nullable").as_deref())
            .unwrap_or_else(|message| {
                self.push_diag(TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE, message);
                false
            });
        let default_expr = attr_value(&attrs, "", "default-expr")
            .or_else(|| attr_value(&attrs, "", "defaultExpr"));
        if default_expr.is_some() {
            self.push_diag(
                TRANSFORM_TEMPLATE_PARAM_DEFAULT_EXPR_RESERVED_CODE,
                format!(
                    "template param `{name}` uses reserved `@default-expr`; use literal `@default` until default expression semantics are defined"
                ),
            );
        }
        let default_value = attr_value(&attrs, "", "default")
            .and_then(|value| self.parse_param_default(&name, value_type, nullable, &value));
        let required = parse_bool_attr(attr_value(&attrs, "", "required").as_deref())
            .unwrap_or_else(|message| {
                self.push_diag(TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE, message);
                false
            });
        Some(TransformTemplateModuleParamDeclaration {
            name,
            value_type,
            nullable,
            default_value,
            required,
            visibility,
        })
    }

    fn lower_template(&mut self, template_id: AstNodeId) {
        let attrs = template_collect_attrs(self.document, template_id);
        let Some(name) = required_attr(&attrs, "name") else {
            self.push_missing_attr("template", "name");
            return;
        };
        let visibility = self.parse_visibility(attr_value(&attrs, "", "visibility").as_deref());
        self.options
            .entrypoints
            .push(TransformTemplateModuleEntrypointDeclaration {
                name: name.clone(),
                visibility,
            });

        let Some(CemAstNode::Element { children, .. }) = self.document.get(template_id) else {
            return;
        };
        for child in children {
            let Some(child_name) = template_element_name(self.document, *child) else {
                continue;
            };
            match child_name {
                "param" => self.lower_param(*child, Some(&name)),
                "body" => self.collect_body_calls(*child, Some(&name)),
                other => self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_UNSUPPORTED_CODE,
                    format!(
                        "`{other}` is not valid inside CEM-native template declaration `{name}`"
                    ),
                ),
            }
        }
    }

    fn lower_output_function(
        &mut self,
        function_id: AstNodeId,
        kind: TransformTemplateOutputFunctionKind,
    ) {
        let element_name = match kind {
            TransformTemplateOutputFunctionKind::Encoding => "encoding-function",
            TransformTemplateOutputFunctionKind::Format => "format-function",
            TransformTemplateOutputFunctionKind::Color => "color-function",
        };
        let attrs = template_collect_attrs(self.document, function_id);
        let Some(name) = required_attr(&attrs, "name") else {
            self.push_missing_attr(element_name, "name");
            return;
        };
        let Some(category) = required_attr(&attrs, "category") else {
            self.push_missing_attr(element_name, "category");
            return;
        };
        let Some(subject) = required_attr(&attrs, "subject") else {
            self.push_missing_attr(element_name, "subject");
            return;
        };
        let Some(produces_raw) = required_attr(&attrs, "produces") else {
            self.push_missing_attr(element_name, "produces");
            return;
        };
        let Some(content_type) = required_attr(&attrs, "content-type") else {
            self.push_missing_attr(element_name, "content-type");
            return;
        };
        let Some(schema) = required_attr(&attrs, "schema") else {
            self.push_missing_attr(element_name, "schema");
            return;
        };
        let Some(produces) = TransformTemplateOutputProducedKind::parse(&produces_raw) else {
            self.push_diag(
                TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE,
                format!(
                    "`{element_name}` `{name}` has unsupported `@produces` value `{produces_raw}`"
                ),
            );
            return;
        };
        let implementation = match optional_trimmed_attr(&attrs, "implementation").as_deref() {
            None => TransformTemplateOutputFunctionImplementation::Cemt,
            Some(value) => {
                if let Some(implementation) =
                    TransformTemplateOutputFunctionImplementation::parse(value)
                {
                    implementation
                } else {
                    self.push_diag(
                        TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE,
                        format!(
                            "`{element_name}` `{name}` has unsupported `@implementation` value `{value}`"
                        ),
                    );
                    TransformTemplateOutputFunctionImplementation::Cemt
                }
            }
        };
        let canonical = self.parse_bool_decl_attr(&attrs, "canonical");
        let streamable = self.parse_bool_decl_attr(&attrs, "streamable");
        let deterministic = self.parse_bool_decl_attr(&attrs, "deterministic");
        let trusted = self.parse_bool_decl_attr(&attrs, "trusted");
        let visibility = self.parse_visibility(attr_value(&attrs, "", "visibility").as_deref());
        let capability = optional_trimmed_attr(&attrs, "capability");
        if implementation.requires_capability() && capability.is_none() {
            self.push_diag(
                TRANSFORM_TEMPLATE_DECLARATION_REQUIRED_CODE,
                format!(
                    "`{element_name}` `{name}` with native or external implementation requires `@capability`"
                ),
            );
        }

        let Some(CemAstNode::Element { children, .. }) = self.document.get(function_id) else {
            return;
        };
        let mut params = Vec::new();
        let mut body_declared = false;
        for child in children {
            let Some(child_name) = template_element_name(self.document, *child) else {
                continue;
            };
            match child_name {
                "param" => {
                    if let Some(param) = self.parse_param_declaration(*child) {
                        params.push(param);
                    }
                    self.reject_decl_children(*child, "param");
                }
                "body" => {
                    body_declared = true;
                }
                other => self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_UNSUPPORTED_CODE,
                    format!("`{element_name}` declarations cannot contain `{other}`"),
                ),
            }
        }

        self.options
            .output_functions
            .push(TransformTemplateOutputFunctionDescriptor {
                kind,
                owner: output_function_owner(&name),
                name,
                category,
                subject,
                produces,
                content_type,
                schema,
                canonical,
                streamable,
                visibility,
                implementation,
                profile: optional_trimmed_attr(&attrs, "profile"),
                extends: optional_trimmed_attr(&attrs, "extends"),
                capability,
                deterministic,
                trusted,
                fallback: optional_trimmed_attr(&attrs, "fallback"),
                params,
                body_declared,
            });
    }

    fn collect_body_calls(&mut self, body_id: AstNodeId, owner_entrypoint: Option<&str>) {
        let Some(CemAstNode::Element { children, .. }) = self.document.get(body_id) else {
            return;
        };
        for child in children {
            self.collect_calls_in_subtree(*child, owner_entrypoint);
        }
    }

    fn collect_calls_in_subtree(&mut self, node_id: AstNodeId, owner_entrypoint: Option<&str>) {
        let Some(CemAstNode::Element { children, .. }) = self.document.get(node_id) else {
            return;
        };
        if template_element_name(self.document, node_id) == Some("call") {
            self.lower_call(node_id, owner_entrypoint);
            self.reject_decl_children(node_id, "call");
            return;
        }
        for child in children {
            self.collect_calls_in_subtree(*child, owner_entrypoint);
        }
    }

    fn lower_call(&mut self, call_id: AstNodeId, owner_entrypoint: Option<&str>) {
        let attrs = template_collect_attrs(self.document, call_id);
        let Some(template) = required_attr(&attrs, "template") else {
            self.push_missing_attr("call", "template");
            return;
        };
        self.options.calls.push(TransformTemplateModuleCallSite {
            owner_entrypoint: owner_entrypoint.map(str::to_owned),
            from: optional_trimmed_attr(&attrs, "from"),
            template,
        });
    }

    fn validate_declarations(&mut self) {
        let mut aliases = BTreeSet::new();
        for import in self.options.imports.clone() {
            if !aliases.insert(import.alias.clone()) {
                self.push_diag(
                    TRANSFORM_TEMPLATE_IMPORT_ALIAS_DUPLICATE_CODE,
                    format!(
                        "template module import alias `{}` is declared more than once",
                        import.alias
                    ),
                );
            }
        }

        let mut entrypoints = BTreeSet::new();
        for entrypoint in self.options.entrypoints.clone() {
            if !entrypoints.insert(entrypoint.name.clone()) {
                self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE,
                    format!(
                        "template entrypoint `{}` is declared more than once",
                        entrypoint.name
                    ),
                );
            }
        }

        let mut params = BTreeSet::new();
        for param in self.options.params.clone() {
            if !params.insert(param.name.clone()) {
                self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE,
                    format!("template param `{}` is declared more than once", param.name),
                );
            }
        }

        let mut output_functions = BTreeSet::new();
        for function in self.options.output_functions.clone() {
            if !output_functions.insert(function.key()) {
                self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE,
                    format!(
                        "CEMT output function `{}` is declared more than once for category `{}` and subject `{}`",
                        function.name, function.category, function.subject
                    ),
                );
            }
        }
    }

    fn reject_decl_children(&mut self, ast_id: AstNodeId, parent_name: &str) {
        let Some(CemAstNode::Element { children, .. }) = self.document.get(ast_id) else {
            return;
        };
        for child in children {
            if let Some(name) = template_element_name(self.document, *child) {
                self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_UNSUPPORTED_CODE,
                    format!("`{parent_name}` declarations cannot contain `{name}`"),
                );
            }
        }
    }

    fn parse_visibility(&mut self, value: Option<&str>) -> TransformTemplateModuleVisibility {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("private") => TransformTemplateModuleVisibility::Private,
            Some("public") => TransformTemplateModuleVisibility::Public,
            Some(other) => {
                self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE,
                    format!("unsupported template declaration visibility `{other}`; use `private` or `public`"),
                );
                TransformTemplateModuleVisibility::Private
            }
        }
    }

    fn parse_bool_decl_attr(
        &mut self,
        attrs: &BTreeMap<(String, String), Option<String>>,
        name: &str,
    ) -> bool {
        parse_bool_attr(attr_value(attrs, "", name).as_deref()).unwrap_or_else(|message| {
            self.push_diag(TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE, message);
            false
        })
    }

    fn parse_param_type(&mut self, value: Option<&str>) -> TransformTemplateModuleParamType {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("any") => TransformTemplateModuleParamType::Any,
            Some("string") => TransformTemplateModuleParamType::String,
            Some("boolean") => TransformTemplateModuleParamType::Boolean,
            Some("number") => TransformTemplateModuleParamType::Number,
            Some("integer") => TransformTemplateModuleParamType::Integer,
            Some("array") => TransformTemplateModuleParamType::Array,
            Some("object") => TransformTemplateModuleParamType::Object,
            Some("json") => TransformTemplateModuleParamType::Json,
            Some(other) => {
                self.push_diag(
                    TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE,
                    format!(
                        "unsupported template param type `{other}`; use `any`, `string`, `boolean`, `number`, `integer`, `array`, `object`, or `json`"
                    ),
                );
                TransformTemplateModuleParamType::Any
            }
        }
    }

    fn parse_param_default(
        &mut self,
        name: &str,
        value_type: TransformTemplateModuleParamType,
        nullable: bool,
        value: &str,
    ) -> Option<Value> {
        let default_value = if nullable && value.trim() == "null" {
            Value::Null
        } else {
            match value_type {
                TransformTemplateModuleParamType::Any
                | TransformTemplateModuleParamType::String => Value::String(value.to_owned()),
                TransformTemplateModuleParamType::Boolean => match value.trim() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => {
                        self.push_param_default_invalid(name, value_type, value);
                        return None;
                    }
                },
                TransformTemplateModuleParamType::Number
                | TransformTemplateModuleParamType::Integer
                | TransformTemplateModuleParamType::Array
                | TransformTemplateModuleParamType::Object
                | TransformTemplateModuleParamType::Json => {
                    match serde_json::from_str::<Value>(value) {
                        Ok(parsed) => parsed,
                        Err(_) => {
                            self.push_param_default_invalid(name, value_type, value);
                            return None;
                        }
                    }
                }
            }
        };

        if value_type.accepts(&default_value, nullable) {
            Some(default_value)
        } else {
            self.push_param_default_invalid(name, value_type, value);
            None
        }
    }

    fn push_param_default_invalid(
        &mut self,
        name: &str,
        value_type: TransformTemplateModuleParamType,
        value: &str,
    ) {
        self.push_diag(
            TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE,
            format!(
                "template param `{name}` default `{value}` is not a valid {} value",
                value_type.as_contract_name()
            ),
        );
    }

    fn push_missing_attr(&mut self, element: &str, attr: &str) {
        self.push_diag(
            TRANSFORM_TEMPLATE_DECLARATION_REQUIRED_CODE,
            format!("CEM-native template `{element}` declaration requires `@{attr}`"),
        );
    }

    fn push_diag(&mut self, code: &str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            uri: Some(self.template_uri.to_owned()),
            code: code.to_owned(),
            severity: Severity::Fatal,
            message: message.into(),
            ..Diagnostic::default()
        });
    }
}

fn template_element_name(doc: &CemDocument, node_id: AstNodeId) -> Option<&str> {
    match doc.get(node_id) {
        Some(CemAstNode::Element { expanded_name, .. }) if !expanded_name.local_name.is_empty() => {
            Some(expanded_name.local_name.as_str())
        }
        _ => None,
    }
}

fn template_collect_attrs(
    doc: &CemDocument,
    node_id: AstNodeId,
) -> BTreeMap<(String, String), Option<String>> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = doc.get(node_id) else {
        return attrs;
    };
    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = doc.get(*attr_id)
        else {
            continue;
        };
        attrs.insert(
            (
                expanded_name.namespace_uri.clone(),
                expanded_name.local_name.clone(),
            ),
            value.clone(),
        );
    }
    attrs
}

fn attr_value(
    attrs: &BTreeMap<(String, String), Option<String>>,
    prefix: &str,
    local: &str,
) -> Option<String> {
    attrs
        .get(&(prefix.to_owned(), local.to_owned()))
        .cloned()
        .flatten()
}

fn required_attr(
    attrs: &BTreeMap<(String, String), Option<String>>,
    local: &str,
) -> Option<String> {
    optional_trimmed_attr(attrs, local)
}

fn optional_trimmed_attr(
    attrs: &BTreeMap<(String, String), Option<String>>,
    local: &str,
) -> Option<String> {
    attr_value(attrs, "", local)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn output_function_owner(name: &str) -> Option<String> {
    name.split_once('.')
        .map(|(owner, _)| owner.trim())
        .filter(|owner| !owner.is_empty())
        .map(str::to_owned)
}

fn import_identity_from_attrs(
    attrs: &BTreeMap<(String, String), Option<String>>,
) -> Option<FormatIdentity> {
    let content_type = optional_trimmed_attr(attrs, "content-type")
        .or_else(|| optional_trimmed_attr(attrs, "contentType"));
    let schema = optional_trimmed_attr(attrs, "schema");
    if content_type.is_none() && schema.is_none() {
        return None;
    }
    Some(FormatIdentity {
        content_type,
        schema,
        ..FormatIdentity::default()
    })
}

fn parse_bool_attr(value: Option<&str>) -> Result<bool, String> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(other) => Err(format!(
            "unsupported boolean value `{other}`; use `true` or `false`"
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModuleCacheKey {
    pub adapter_id: String,
    pub resolved_uri: String,
    #[serde(default)]
    pub identity: Option<FormatIdentity>,
    pub content_hash: String,
    pub entrypoint: TransformTemplateEntrypoint,
    pub execution_policy: TransformExecutionPolicy,
    pub dependency_graph_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateResolvedModule {
    pub alias: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_uri: Option<String>,
    pub uri: String,
    #[serde(default)]
    pub identity: Option<FormatIdentity>,
    pub content_hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateModulePreflight {
    #[serde(default)]
    pub resolved_imports: Vec<TransformTemplateResolvedModule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<TransformTemplateModuleCacheKey>,
}

impl TransformTemplateModuleCacheKey {
    pub fn new(
        adapter_id: impl Into<String>,
        resolved_uri: impl Into<String>,
        identity: Option<FormatIdentity>,
        content_hash: impl Into<String>,
        entrypoint: TransformTemplateEntrypoint,
        execution_policy: TransformExecutionPolicy,
        dependency_graph_hash: impl Into<String>,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            resolved_uri: resolved_uri.into(),
            identity,
            content_hash: content_hash.into(),
            entrypoint,
            execution_policy,
            dependency_graph_hash: dependency_graph_hash.into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateCompiledArtifact {
    pub adapter_id: String,
    pub kind: TransformTemplateKind,
    pub template_uri: String,
    pub identity: Option<FormatIdentity>,
    pub entrypoint: TransformTemplateEntrypoint,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub opaque: Value,
    #[serde(skip)]
    native_payload: Option<Arc<dyn Any + Send + Sync>>,
}

impl fmt::Debug for TransformTemplateCompiledArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransformTemplateCompiledArtifact")
            .field("adapter_id", &self.adapter_id)
            .field("kind", &self.kind)
            .field("template_uri", &self.template_uri)
            .field("identity", &self.identity)
            .field("entrypoint", &self.entrypoint)
            .field("opaque", &self.opaque)
            .field("has_native_payload", &self.native_payload.is_some())
            .finish()
    }
}

impl PartialEq for TransformTemplateCompiledArtifact {
    fn eq(&self, other: &Self) -> bool {
        self.adapter_id == other.adapter_id
            && self.kind == other.kind
            && self.template_uri == other.template_uri
            && self.identity == other.identity
            && self.entrypoint == other.entrypoint
            && self.opaque == other.opaque
    }
}

impl TransformTemplateCompiledArtifact {
    pub fn new(
        adapter_id: impl Into<String>,
        kind: TransformTemplateKind,
        template_uri: impl Into<String>,
        identity: Option<FormatIdentity>,
        entrypoint: TransformTemplateEntrypoint,
        opaque: Value,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            kind,
            template_uri: template_uri.into(),
            identity,
            entrypoint,
            opaque,
            native_payload: None,
        }
    }

    pub fn with_native_payload<T>(mut self, payload: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.native_payload = Some(Arc::new(payload));
        self
    }

    pub fn native_payload<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.native_payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<T>())
    }
}

#[derive(Debug, Clone)]
pub struct TransformTemplateCompileResponse {
    pub artifact: TransformTemplateCompiledArtifact,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateDataArtifact {
    pub artifact_id: String,
    pub uri: Option<String>,
    pub identity: Option<FormatIdentity>,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct TransformTemplateRenderRequest<'a> {
    pub compiled: &'a TransformTemplateCompiledArtifact,
    pub primary_input: &'a TransformTemplateDataArtifact,
    pub secondary_inputs: &'a BTreeMap<String, TransformTemplateDataArtifact>,
    pub target: Option<&'a FormatIdentity>,
    pub target_scope: &'a ScopeConfig,
    pub execution_policy: TransformExecutionPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformTemplateOutputArtifact {
    pub uri: Option<String>,
    pub identity: Option<FormatIdentity>,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_spans: Vec<OutputSpan>,
}

#[derive(Debug, Clone)]
pub struct TransformTemplateRenderResponse {
    pub output: TransformTemplateOutputArtifact,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformTemplateAdapterExecutionPhase {
    Compile,
    Render,
}

impl TransformTemplateAdapterExecutionPhase {
    pub fn diagnostic_origin(self) -> TransformDiagnosticOrigin {
        match self {
            Self::Compile => TransformDiagnosticOrigin::TemplateCompile,
            Self::Render => TransformDiagnosticOrigin::TemplateExecution,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Render => "render",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformTemplateAdapterError {
    NotImplemented {
        adapter_id: &'static str,
        phase: TransformTemplateAdapterExecutionPhase,
    },
    Failed {
        adapter_id: &'static str,
        phase: TransformTemplateAdapterExecutionPhase,
        message: String,
    },
}

impl TransformTemplateAdapterError {
    pub const NOT_IMPLEMENTED_CODE: &'static str = "cem.transform_template.adapter_not_implemented";
    pub const FAILED_CODE: &'static str = "cem.transform_template.adapter_failed";

    pub fn not_implemented(
        adapter_id: &'static str,
        phase: TransformTemplateAdapterExecutionPhase,
    ) -> Self {
        Self::NotImplemented { adapter_id, phase }
    }

    pub fn failed(
        adapter_id: &'static str,
        phase: TransformTemplateAdapterExecutionPhase,
        message: impl Into<String>,
    ) -> Self {
        Self::Failed {
            adapter_id,
            phase,
            message: message.into(),
        }
    }

    pub fn adapter_id(&self) -> &'static str {
        match self {
            Self::NotImplemented { adapter_id, .. } | Self::Failed { adapter_id, .. } => adapter_id,
        }
    }

    pub fn phase(&self) -> TransformTemplateAdapterExecutionPhase {
        match self {
            Self::NotImplemented { phase, .. } | Self::Failed { phase, .. } => *phase,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::NotImplemented { .. } => Self::NOT_IMPLEMENTED_CODE,
            Self::Failed { .. } => Self::FAILED_CODE,
        }
    }

    pub fn diagnostic_origin(&self) -> TransformDiagnosticOrigin {
        self.phase().diagnostic_origin()
    }

    pub fn diagnostic(&self, uri: Option<&str>) -> Diagnostic {
        Diagnostic {
            uri: uri.map(str::to_owned),
            code: self.code().to_owned(),
            severity: Severity::Fatal,
            message: self.to_string(),
            ..Diagnostic::default()
        }
    }
}

impl fmt::Display for TransformTemplateAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented { adapter_id, phase } => write!(
                f,
                "transform template adapter `{adapter_id}` does not implement {}",
                phase.label()
            ),
            Self::Failed {
                adapter_id,
                phase,
                message,
            } => write!(
                f,
                "transform template adapter `{adapter_id}` failed during {}: {message}",
                phase.label()
            ),
        }
    }
}

impl std::error::Error for TransformTemplateAdapterError {}

pub type TransformTemplateAdapterResult<T> = Result<T, TransformTemplateAdapterError>;

pub trait TransformTemplateAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn kind(&self) -> TransformTemplateKind;
    fn capability(&self) -> TransformTemplateAdapterCapability {
        TransformTemplateAdapterCapability::SelectorOnly
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool;

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        let _ = request;
        Err(TransformTemplateAdapterError::not_implemented(
            self.id(),
            TransformTemplateAdapterExecutionPhase::Compile,
        ))
    }

    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        let _ = request;
        Err(TransformTemplateAdapterError::not_implemented(
            self.id(),
            TransformTemplateAdapterExecutionPhase::Render,
        ))
    }
}

#[derive(Clone, Default)]
pub struct TransformTemplateAdapterRegistry {
    adapters: Vec<Arc<dyn TransformTemplateAdapter>>,
}

impl fmt::Debug for TransformTemplateAdapterRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransformTemplateAdapterRegistry")
            .field("adapter_count", &self.adapters.len())
            .finish()
    }
}

impl TransformTemplateAdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_adapters() -> Self {
        let mut registry = Self::new();
        registry.register(StaticTransformTemplateAdapter::new(
            "cem-native-template",
            TransformTemplateKind::CemNative,
            &[
                "application/cem+xml",
                "application/cem",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                CEM_TRANSFORM_CONTENT_TYPE,
                "text/cem",
                "text/cem-ml",
            ],
            &[
                CEM_NATIVE_TEMPLATE_SCHEMA_URI,
                CEM_TRANSFORM_SCHEMA_URI,
                crate::schema::ir::CEM_CORE_NAMESPACE,
            ],
            &[crate::schema::ir::CEM_CORE_NAMESPACE],
        ));
        registry.register(StaticTransformTemplateAdapter::new(
            "xslt-template",
            TransformTemplateKind::Xslt,
            crate::legacy_custom_element::TEMPLATE_CONTENT_TYPES,
            &[XSLT_SCHEMA_URI],
            &[crate::schema::xslt::XSL_NAMESPACE],
        ));
        registry
    }

    pub fn register(&mut self, adapter: impl TransformTemplateAdapter + 'static) {
        self.adapters.push(Arc::new(adapter));
    }

    pub fn register_arc(&mut self, adapter: Arc<dyn TransformTemplateAdapter>) {
        self.adapters.push(adapter);
    }

    pub fn select(&self, identity: &FormatIdentity) -> TransformTemplateAdapterResolution {
        let matches = self
            .adapters
            .iter()
            .filter(|adapter| adapter.matches_template(identity))
            .cloned()
            .collect::<Vec<_>>();
        let candidates = preferred_adapter_matches(&matches);
        let selections = candidates
            .iter()
            .map(|adapter| TransformTemplateAdapterSelection {
                adapter_id: adapter.id(),
                kind: adapter.kind(),
            })
            .collect::<Vec<_>>();

        match selections.as_slice() {
            [selection] => TransformTemplateAdapterResolution::Matched(selection.clone()),
            [] => TransformTemplateAdapterResolution::Unsupported,
            many => TransformTemplateAdapterResolution::Ambiguous(
                many.iter().map(|selection| selection.adapter_id).collect(),
            ),
        }
    }

    pub fn select_adapter(&self, identity: &FormatIdentity) -> TransformTemplateAdapterLookup {
        let matches = self
            .adapters
            .iter()
            .filter(|adapter| adapter.matches_template(identity))
            .cloned()
            .collect::<Vec<_>>();
        let candidates = preferred_adapter_matches(&matches);

        match candidates.as_slice() {
            [adapter] => TransformTemplateAdapterLookup::Matched(Arc::clone(adapter)),
            [] => TransformTemplateAdapterLookup::Unsupported,
            many => TransformTemplateAdapterLookup::Ambiguous(
                many.iter().map(|adapter| adapter.id()).collect(),
            ),
        }
    }
}

fn preferred_adapter_matches(
    matches: &[Arc<dyn TransformTemplateAdapter>],
) -> Vec<&Arc<dyn TransformTemplateAdapter>> {
    let executable = matches
        .iter()
        .filter(|adapter| adapter.capability() == TransformTemplateAdapterCapability::Executable)
        .collect::<Vec<_>>();
    if executable.is_empty() {
        matches.iter().collect()
    } else {
        executable
    }
}

#[derive(Debug, Clone)]
pub struct StaticTransformTemplateAdapter {
    id: &'static str,
    kind: TransformTemplateKind,
    content_types: Vec<String>,
    schemas: Vec<String>,
    namespaces: Vec<String>,
}

impl StaticTransformTemplateAdapter {
    pub fn new(
        id: &'static str,
        kind: TransformTemplateKind,
        content_types: &[&str],
        schemas: &[&str],
        namespaces: &[&str],
    ) -> Self {
        Self {
            id,
            kind,
            content_types: content_types
                .iter()
                .map(|content_type| content_type_essence(content_type))
                .collect(),
            schemas: schemas
                .iter()
                .map(|schema| schema.trim().to_owned())
                .collect(),
            namespaces: namespaces
                .iter()
                .map(|namespace| namespace.trim().to_owned())
                .collect(),
        }
    }
}

impl TransformTemplateAdapter for StaticTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        self.id
    }

    fn kind(&self) -> TransformTemplateKind {
        self.kind
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        if let Some(content_type) = identity.content_type.as_deref() {
            return self
                .content_types
                .iter()
                .any(|allowed| allowed == &content_type_essence(content_type));
        }

        let schema = identity
            .schema
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        if !schema.is_empty() {
            return self.schemas.iter().any(|allowed| allowed == schema);
        }

        identity
            .default_namespace
            .as_deref()
            .is_some_and(|uri| self.namespaces.iter().any(|allowed| allowed == uri))
            || identity
                .namespaces
                .values()
                .any(|uri| self.namespaces.iter().any(|allowed| allowed == uri))
    }
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn template_input(uri: &str, source: &str, identity: Option<FormatIdentity>) -> TemplateInput {
        TemplateInput {
            uri: uri.to_owned(),
            bytes: source.as_bytes().to_vec(),
            identity,
            root_scope: ScopeConfig::default(),
        }
    }

    #[test]
    fn builtins_select_cem_native_and_xslt_template_adapters() {
        let registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
        let cem = FormatIdentity {
            content_type: Some("text/cem-ml; charset=utf-8".to_owned()),
            ..FormatIdentity::default()
        };
        let native_template = FormatIdentity {
            content_type: Some(format!("{CEM_NATIVE_TEMPLATE_CONTENT_TYPE}; charset=utf-8")),
            ..FormatIdentity::default()
        };
        let transform = FormatIdentity {
            content_type: Some(format!("{CEM_TRANSFORM_CONTENT_TYPE}; charset=utf-8")),
            ..FormatIdentity::default()
        };
        let xslt = FormatIdentity {
            default_namespace: Some(crate::schema::xslt::XSL_NAMESPACE.to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&cem),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template",
                kind: TransformTemplateKind::CemNative,
            })
        );
        assert_eq!(
            registry.select(&native_template),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template",
                kind: TransformTemplateKind::CemNative,
            })
        );
        assert_eq!(
            registry.select(&transform),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template",
                kind: TransformTemplateKind::CemNative,
            })
        );
        assert_eq!(
            registry.select(&xslt),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "xslt-template",
                kind: TransformTemplateKind::Xslt,
            })
        );
    }

    #[test]
    fn native_template_module_options_model_imports_visibility_params_and_limits() {
        let import: TransformTemplateModuleImport = serde_json::from_value(json!({
            "alias": "ui",
            "uri": "templates/ui.cem"
        }))
        .expect("import defaults");
        let entrypoint: TransformTemplateModuleEntrypointDeclaration =
            serde_json::from_value(json!({"name": "card"})).expect("entrypoint defaults");
        let param: TransformTemplateModuleParamDeclaration = serde_json::from_value(json!({
            "name": "locale",
            "type": "string",
            "nullable": true,
            "defaultValue": "en-US"
        }))
        .expect("param defaults");
        let options = TransformTemplateModuleOptions {
            imports: vec![import],
            entrypoints: vec![entrypoint],
            params: vec![param],
            calls: Vec::new(),
            output_functions: Vec::new(),
            limits: TransformTemplateModuleLimits::default(),
        };

        assert_eq!(
            options.imports[0].kind,
            TransformTemplateModuleDependencyKind::Import
        );
        assert_eq!(
            options.entrypoints[0].visibility,
            TransformTemplateModuleVisibility::Private
        );
        assert_eq!(
            options.params[0].visibility,
            TransformTemplateModuleVisibility::Private
        );
        assert_eq!(
            options.params[0].value_type,
            TransformTemplateModuleParamType::String
        );
        assert!(options.params[0].nullable);
        assert!(!options.params[0].required);
        assert_eq!(options.limits.max_import_depth, 32);
        assert_eq!(options.limits.max_recursion_depth, 64);
    }

    #[test]
    fn cem_native_template_schema_shape_declares_module_surface() {
        let module = CEM_NATIVE_TEMPLATE_SCHEMA_ELEMENTS
            .iter()
            .find(|element| element.local_name == "module")
            .expect("module schema");
        let import = CEM_NATIVE_TEMPLATE_SCHEMA_ELEMENTS
            .iter()
            .find(|element| element.local_name == "import")
            .expect("import schema");
        let template = CEM_NATIVE_TEMPLATE_SCHEMA_ELEMENTS
            .iter()
            .find(|element| element.local_name == "template")
            .expect("template schema");
        let call = CEM_NATIVE_TEMPLATE_SCHEMA_ELEMENTS
            .iter()
            .find(|element| element.local_name == "call")
            .expect("call schema");
        let param = CEM_NATIVE_TEMPLATE_SCHEMA_ELEMENTS
            .iter()
            .find(|element| element.local_name == "param")
            .expect("param schema");

        assert_eq!(
            CEM_NATIVE_TEMPLATE_SCHEMA_URI,
            CEM_NATIVE_TEMPLATE_NAMESPACE_URI
        );
        assert_eq!(
            module.child_elements,
            &["import", "param", "template", "body"]
        );
        assert_eq!(import.required_attributes, &["as", "src"]);
        assert!(import.optional_attributes.contains(&"content-type"));
        assert!(param.optional_attributes.contains(&"type"));
        assert!(param.optional_attributes.contains(&"nullable"));
        assert_eq!(template.required_attributes, &["name"]);
        assert!(template.optional_attributes.contains(&"visibility"));
        assert_eq!(call.required_attributes, &["template"]);
        assert!(call.optional_attributes.contains(&"from"));
        assert!(call.optional_attributes.contains(&"with:*"));
    }

    #[test]
    fn cem_native_template_schema_artifact_matches_shape_table() {
        let artifact = include_str!(
            "../schema-packages/cem-native-template/v1/schema/cem-native-template.cem"
        );

        assert!(artifact.contains(CEM_NATIVE_TEMPLATE_SCHEMA_URI));
        assert!(artifact.contains(CEM_NATIVE_TEMPLATE_NAMESPACE_URI));
        for element in CEM_NATIVE_TEMPLATE_SCHEMA_ELEMENTS {
            assert!(
                artifact.contains(&format!("{{element @name=\"{}\"", element.local_name)),
                "artifact should document `{}`",
                element.local_name
            );
            for attribute in element
                .required_attributes
                .iter()
                .chain(element.optional_attributes.iter())
            {
                assert!(
                    artifact.contains(&format!("@name=\"{attribute}\"")),
                    "artifact should document `{}` attribute on `{}`",
                    attribute,
                    element.local_name
                );
            }
            for child in element.child_elements {
                assert!(
                    artifact.contains(child),
                    "artifact should document `{}` child on `{}`",
                    child,
                    element.local_name
                );
            }
        }
    }

    #[test]
    fn builtins_accept_cem_native_template_schema_identity() {
        let registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
        let identity = FormatIdentity {
            schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            default_namespace: Some(CEM_NATIVE_TEMPLATE_NAMESPACE_URI.to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&identity),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template",
                kind: TransformTemplateKind::CemNative,
            })
        );
    }

    #[test]
    fn builtins_accept_cem_transform_schema_identity() {
        let registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
        let identity = FormatIdentity {
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&identity),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template",
                kind: TransformTemplateKind::CemNative,
            })
        );
    }

    #[test]
    fn cem_native_template_module_parser_lowers_declarations() {
        let response = parse_cem_native_template_module_options(
            TransformTemplateModuleParseRequest {
                template: template_input(
                    "templates/page.cem",
                    r#"{@doc cem-ml 1}
{module @version="1" |
  {import @as="ui" @src="ui.cem" @content-type="text/cem-ml" @schema="https://cem.dev/ns/template/cem-native/1"}
  {param @name="locale" @default="en-US" @visibility="public"}
  {param @name="enabled" @type="boolean" @default="true"}
  {param @name="subtitle" @type="string" @nullable="true" @default="null"}
  {template @name="card" @visibility="public" |
    {param @name="title" @type="string" @required="true"}
    {param @name="count" @type="integer" @default="3"}
    {body | {article | {$title} {call @template="badge"} {call @from="ui" @template="icon"}}}
  }
}"#,
                    Some(FormatIdentity {
                        schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            },
        );

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(response.module_declared);
        assert_eq!(response.module_options.imports.len(), 1);
        assert_eq!(response.module_options.imports[0].alias, "ui");
        assert_eq!(response.module_options.imports[0].uri, "ui.cem");
        assert_eq!(
            response.module_options.imports[0]
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/cem-ml")
        );
        assert_eq!(
            response.module_options.entrypoints,
            vec![TransformTemplateModuleEntrypointDeclaration {
                name: "card".to_owned(),
                visibility: TransformTemplateModuleVisibility::Public,
            }]
        );
        assert_eq!(response.module_options.params.len(), 5);
        assert_eq!(response.module_options.params[0].name, "locale");
        assert_eq!(
            response.module_options.params[0].default_value,
            Some(Value::String("en-US".to_owned()))
        );
        assert_eq!(
            response.module_options.params[0].visibility,
            TransformTemplateModuleVisibility::Public
        );
        assert_eq!(response.module_options.params[1].name, "enabled");
        assert_eq!(
            response.module_options.params[1].value_type,
            TransformTemplateModuleParamType::Boolean
        );
        assert_eq!(
            response.module_options.params[1].default_value,
            Some(Value::Bool(true))
        );
        assert_eq!(response.module_options.params[2].name, "subtitle");
        assert_eq!(
            response.module_options.params[2].value_type,
            TransformTemplateModuleParamType::String
        );
        assert!(response.module_options.params[2].nullable);
        assert_eq!(
            response.module_options.params[2].default_value,
            Some(Value::Null)
        );
        assert_eq!(response.module_options.params[3].name, "card.title");
        assert_eq!(
            response.module_options.params[3].value_type,
            TransformTemplateModuleParamType::String
        );
        assert!(response.module_options.params[3].required);
        assert_eq!(response.module_options.params[4].name, "card.count");
        assert_eq!(
            response.module_options.params[4].value_type,
            TransformTemplateModuleParamType::Integer
        );
        assert_eq!(
            response.module_options.params[4].default_value,
            Some(Value::Number(3.into()))
        );
        assert_eq!(
            response.module_options.calls,
            vec![
                TransformTemplateModuleCallSite {
                    owner_entrypoint: Some("card".to_owned()),
                    from: None,
                    template: "badge".to_owned(),
                },
                TransformTemplateModuleCallSite {
                    owner_entrypoint: Some("card".to_owned()),
                    from: Some("ui".to_owned()),
                    template: "icon".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn cemt_module_parser_lowers_output_function_declarations() {
        let response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template_input(
                    "schema-packages/cem-transform/v1/examples/function-declarations.cemt",
                    include_str!(
                        "../schema-packages/cem-transform/v1/examples/function-declarations.cemt"
                    ),
                    Some(FormatIdentity {
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            });

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(response.module_declared);
        assert_eq!(response.module_options.output_functions.len(), 4);
        let html_text = &response.module_options.output_functions[0];
        assert_eq!(
            html_text.kind,
            TransformTemplateOutputFunctionKind::Encoding
        );
        assert_eq!(html_text.owner.as_deref(), Some("html"));
        assert_eq!(html_text.name, "html.text");
        assert_eq!(html_text.category, "html-text");
        assert_eq!(
            html_text.produces,
            TransformTemplateOutputProducedKind::Text
        );
        assert_eq!(html_text.content_type, "text/html");
        assert_eq!(html_text.schema, "https://cem.dev/ns/data/html/1");
        assert!(html_text.canonical);
        assert!(html_text.streamable);
        assert_eq!(html_text.params.len(), 2);

        let custom = response
            .module_options
            .output_functions
            .iter()
            .find(|function| function.name == "acme.markdown.callout-block")
            .expect("custom function declaration");
        assert_eq!(custom.owner.as_deref(), Some("acme"));
        assert_eq!(
            custom.implementation,
            TransformTemplateOutputFunctionImplementation::Cemt
        );
        assert_eq!(custom.visibility, TransformTemplateModuleVisibility::Public);
        assert!(custom.deterministic);
        assert_eq!(custom.extends.as_deref(), Some("markdown-document"));
        assert!(custom.body_declared);
    }

    #[test]
    fn cemt_module_parser_reports_output_function_declaration_errors() {
        let response = parse_cem_native_template_module_options(
            TransformTemplateModuleParseRequest {
                template: template_input(
                    "templates/bad-functions.cemt",
                    r#"{@doc cem-ml 1}
{module |
  {encoding-function @name="html.text" @category="html-text" @subject="string" @produces="text" @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
  {encoding-function @name="html.text" @category="html-text" @subject="string" @produces="text" @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
  {format-function @name="acme.native.json" @category="json-document" @subject="object" @produces="tokens" @content-type="application/json" @schema="https://acme.test/ns/api/json/1" @implementation="native"}
  {color-function @name="broken.color" @category="terminal-color" @subject="tokens" @produces="paint" @content-type="text/plain" @schema="https://cem.dev/ns/data/text/terminal/1"}
}"#,
                    Some(FormatIdentity {
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            },
        );

        assert!(response.module_declared);
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_DECLARATION_REQUIRED_CODE));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE));
    }

    #[test]
    fn output_function_registry_resolves_identity_and_capabilities() {
        let response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template_input(
                    "schema-packages/cem-transform/v1/examples/function-declarations.cemt",
                    include_str!(
                        "../schema-packages/cem-transform/v1/examples/function-declarations.cemt"
                    ),
                    Some(FormatIdentity {
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            });
        let mut registry =
            TransformTemplateOutputFunctionRegistry::from_module_options(&response.module_options);
        let query = TransformTemplateOutputFunctionQuery::for_identity(
            TransformTemplateOutputFunctionKind::Encoding,
            "text/html; charset=utf-8",
            "https://cem.dev/ns/data/html/1",
            "html-text",
            "string",
        );
        let resolved = registry
            .resolve(&query, &BTreeSet::new())
            .expect("html text function resolves");
        assert_eq!(resolved.name, "html.text");

        let mut native = resolved.clone();
        native.name = "acme.native.html-text".to_owned();
        native.owner = Some("acme".to_owned());
        native.implementation = TransformTemplateOutputFunctionImplementation::Native;
        native.capability = Some("acme.native.HtmlTextEncoder".to_owned());
        registry.register(native.clone());
        let ambiguous = registry
            .resolve(&query, &BTreeSet::new())
            .expect_err("two matching encoders should be ambiguous");
        assert!(matches!(
            ambiguous,
            TransformTemplateOutputFunctionResolutionError::Ambiguous { .. }
        ));

        let native_query = TransformTemplateOutputFunctionQuery {
            name: Some(native.name.clone()),
            ..query.clone()
        };
        let missing = registry
            .resolve(&native_query, &BTreeSet::new())
            .expect_err("native function requires host capability");
        assert!(matches!(
            missing,
            TransformTemplateOutputFunctionResolutionError::MissingCapability { .. }
        ));
        assert_eq!(
            missing.diagnostic(Some("template.cemt")).code,
            TRANSFORM_TEMPLATE_OUTPUT_FUNCTION_CAPABILITY_MISSING_CODE
        );

        let mut capabilities = BTreeSet::new();
        capabilities.insert("acme.native.HtmlTextEncoder".to_owned());
        let resolved_native = registry
            .resolve(&native_query, &capabilities)
            .expect("native function resolves with capability");
        assert_eq!(resolved_native.name, "acme.native.html-text");
    }

    #[test]
    fn cem_native_template_module_parser_ignores_plain_fragments_without_schema() {
        let response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template_input(
                    "templates/fragment.cem",
                    r#"{span | {$datadom.attributes.label}}"#,
                    Some(FormatIdentity {
                        content_type: Some("text/cem-ml".to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            });

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(!response.module_declared);
        assert!(response.module_options.imports.is_empty());
        assert!(response.module_options.entrypoints.is_empty());
        assert!(response.module_options.params.is_empty());
    }

    #[test]
    fn cem_native_template_module_parser_reports_declaration_errors() {
        let response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template_input(
                    "templates/bad.cem",
                    r#"{@doc cem-ml 1}
{module |
  {include @src="legacy.cem"}
  {import @as="ui" @src="ui.cem"}
  {import @as="ui" @src="ui-2.cem"}
  {param @name="locale" @required="maybe"}
  {param @name="subtitle" @nullable="maybe"}
  {param @name="count" @type="integer" @default="1.5"}
  {param @name="dynamic" @default-expr="input.title"}
  {param @name="mode" @type="token"}
  {template @name="card"}
  {template @name="card"}
}"#,
                    Some(FormatIdentity {
                        schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            });

        assert!(response.module_declared);
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_INCLUDE_RESERVED_CODE));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_DEFAULT_EXPR_RESERVED_CODE));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_IMPORT_ALIAS_DUPLICATE_CODE));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE));
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE));
    }

    #[test]
    fn cem_native_template_module_parser_trims_structural_declaration_values() {
        let response = parse_cem_native_template_module_options(
            TransformTemplateModuleParseRequest {
                template: template_input(
                    "templates/spaced.cem",
                    r#"{@doc cem-ml 1}
{module |
  {import @as=" ui " @src=" partials/ui.cem " @content-type=" text/cem-ml " @schema=" https://cem.dev/ns/template/cem-native/1 "}
  {param @name=" locale " @default=" en-US "}
  {template @name=" card " @visibility=" public " |
    {param @name=" title " @default=" Untitled "}
    {body | {article | {call @from=" ui " @template=" icon "}}}
  }
}"#,
                    Some(FormatIdentity {
                        schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            },
        );

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.module_options.imports[0].alias, "ui");
        assert_eq!(response.module_options.imports[0].uri, "partials/ui.cem");
        let import_identity = response.module_options.imports[0]
            .identity
            .as_ref()
            .expect("import identity");
        assert_eq!(import_identity.content_type.as_deref(), Some("text/cem-ml"));
        assert_eq!(
            import_identity.schema.as_deref(),
            Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI)
        );
        assert_eq!(response.module_options.params[0].name, "locale");
        assert_eq!(
            response.module_options.params[0].default_value,
            Some(Value::String(" en-US ".to_owned()))
        );
        assert_eq!(
            response.module_options.entrypoints[0].name,
            "card".to_owned()
        );
        assert_eq!(response.module_options.params[1].name, "card.title");
        assert_eq!(
            response.module_options.params[1].default_value,
            Some(Value::String(" Untitled ".to_owned()))
        );
        assert_eq!(
            response.module_options.calls,
            vec![TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: Some("ui".to_owned()),
                template: "icon".to_owned(),
            }]
        );
    }

    #[test]
    fn cem_native_template_schema_requires_module_root() {
        let response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template_input(
                    "templates/not-module.cem",
                    r#"{article | {$title}}"#,
                    Some(FormatIdentity {
                        schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                ),
            });

        assert!(!response.module_declared);
        assert!(response
            .diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_DECLARATION_REQUIRED_CODE));
    }

    #[test]
    fn native_template_module_cache_key_records_identity_policy_entrypoint_and_dependencies() {
        let identity = FormatIdentity {
            content_type: Some("application/vnd.cem.template+cem;version=2".to_owned()),
            ..FormatIdentity::default()
        };
        let key = TransformTemplateModuleCacheKey::new(
            "cem-native-template-v2",
            "file:///workspace/templates/card.cem",
            Some(identity),
            "sha256:template",
            TransformTemplateEntrypoint::named("card"),
            TransformExecutionPolicy::default(),
            "sha256:dependency-graph",
        );
        let json = serde_json::to_value(&key).expect("cache key serializes");

        assert_eq!(json["adapterId"], "cem-native-template-v2");
        assert_eq!(json["resolvedUri"], "file:///workspace/templates/card.cem");
        assert_eq!(json["contentHash"], "sha256:template");
        assert_eq!(json["entrypoint"]["name"], "card");
        assert_eq!(json["executionPolicy"]["failurePolicy"], "fail-fast");
        assert_eq!(json["dependencyGraphHash"], "sha256:dependency-graph");
    }

    #[test]
    fn runtime_adapter_can_claim_new_cem_native_template_schema() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(StaticTransformTemplateAdapter::new(
            "cem-native-template-v2",
            TransformTemplateKind::CemNative,
            &["application/vnd.cem.template+cem;version=2"],
            &["https://cem.dev/ns/template/cem-native/2"],
            &[],
        ));
        let identity = FormatIdentity {
            schema: Some("https://cem.dev/ns/template/cem-native/2".to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&identity),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "cem-native-template-v2",
                kind: TransformTemplateKind::CemNative,
            })
        );
    }

    #[test]
    fn ambiguous_template_adapter_matches_are_reported() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(StaticTransformTemplateAdapter::new(
            "one",
            TransformTemplateKind::CemNative,
            &["text/cem-ml"],
            &[],
            &[],
        ));
        registry.register(StaticTransformTemplateAdapter::new(
            "two",
            TransformTemplateKind::CemNative,
            &["text/cem-ml"],
            &[],
            &[],
        ));
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&identity),
            TransformTemplateAdapterResolution::Ambiguous(vec!["one", "two"])
        );
    }

    #[test]
    fn builtin_template_adapter_compile_and_render_are_reserved_by_default() {
        let registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let adapter = match registry.select_adapter(&identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => adapter,
            other => panic!("expected matched adapter, got {other:?}"),
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: b"{ $title }".to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compile_error = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect_err("static adapter should not compile templates");

        assert_eq!(
            compile_error.code(),
            TransformTemplateAdapterError::NOT_IMPLEMENTED_CODE
        );
        assert_eq!(
            compile_error.diagnostic_origin(),
            TransformDiagnosticOrigin::TemplateCompile
        );
        assert_eq!(
            compile_error.diagnostic(Some("template.cem")).uri,
            Some("template.cem".to_owned())
        );

        let compiled = TransformTemplateCompiledArtifact::new(
            adapter.id(),
            adapter.kind(),
            template.uri.clone(),
            template.identity.clone(),
            TransformTemplateEntrypoint::implicit(),
            Value::Null,
        );
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.xml".to_owned()),
            identity: None,
            value: json!({"title": "Example"}),
        };
        let secondary_inputs = BTreeMap::new();
        let render_error = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect_err("static adapter should not render templates");

        assert_eq!(
            render_error.diagnostic_origin(),
            TransformDiagnosticOrigin::TemplateExecution
        );
    }

    #[derive(Debug)]
    struct RuntimeAdapter;

    impl TransformTemplateAdapter for RuntimeAdapter {
        fn id(&self) -> &'static str {
            "runtime-cem-native-template"
        }

        fn kind(&self) -> TransformTemplateKind {
            TransformTemplateKind::CemNative
        }

        fn capability(&self) -> TransformTemplateAdapterCapability {
            TransformTemplateAdapterCapability::Executable
        }

        fn matches_template(&self, identity: &FormatIdentity) -> bool {
            identity.content_type.as_deref() == Some("application/vnd.cem.template+cem;version=2")
        }

        fn compile(
            &self,
            request: TransformTemplateCompileRequest<'_>,
        ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
            Ok(TransformTemplateCompileResponse {
                artifact: TransformTemplateCompiledArtifact::new(
                    self.id(),
                    self.kind(),
                    request.template.uri.clone(),
                    request.template.identity.clone(),
                    request.entrypoint.clone(),
                    json!({
                        "bytes": request.template.bytes.len(),
                        "moduleImports": request.module_options.imports.len(),
                        "moduleEntrypoints": request.module_options.entrypoints.len(),
                        "params": request.params.len(),
                    }),
                )
                .with_native_payload("compiled-runtime-template"),
                diagnostics: Vec::new(),
            })
        }

        fn render(
            &self,
            request: TransformTemplateRenderRequest<'_>,
        ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
            Ok(TransformTemplateRenderResponse {
                output: TransformTemplateOutputArtifact {
                    uri: None,
                    identity: request.target.cloned(),
                    value: json!({
                        "adapter": request.compiled.adapter_id,
                        "primary": request.primary_input.value,
                        "secondaryInputs": request.secondary_inputs.len(),
                    }),
                    source_map: None,
                    output_spans: Vec::new(),
                },
                diagnostics: Vec::new(),
            })
        }
    }

    #[test]
    fn runtime_template_adapter_can_compile_and_render_through_registry_selection() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(RuntimeAdapter);
        let identity = FormatIdentity {
            content_type: Some("application/vnd.cem.template+cem;version=2".to_owned()),
            ..FormatIdentity::default()
        };
        let adapter = match registry.select_adapter(&identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => adapter,
            other => panic!("expected matched adapter, got {other:?}"),
        };
        let template = TemplateInput {
            uri: "template-v2.cem".to_owned(),
            bytes: b"{ $title }".to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("runtime adapter should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.xml".to_owned()),
            identity: None,
            value: json!({"title": "Example"}),
        };
        let secondary_inputs = BTreeMap::new();
        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("runtime adapter should render");

        assert_eq!(compiled.adapter_id, "runtime-cem-native-template");
        assert_eq!(
            rendered.output.value,
            json!({
                "adapter": "runtime-cem-native-template",
                "primary": {"title": "Example"},
                "secondaryInputs": 0
            })
        );
    }

    #[test]
    fn runtime_template_adapter_receives_module_options_during_compile() {
        let mut registry = TransformTemplateAdapterRegistry::new();
        registry.register(RuntimeAdapter);
        let identity = FormatIdentity {
            content_type: Some("application/vnd.cem.template+cem;version=2".to_owned()),
            ..FormatIdentity::default()
        };
        let adapter = match registry.select_adapter(&identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => adapter,
            other => panic!("expected matched adapter, got {other:?}"),
        };
        let template = TemplateInput {
            uri: "template-v2.cem".to_owned(),
            bytes: b"{ $title }".to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let module_options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
            }],
            entrypoints: vec![TransformTemplateModuleEntrypointDeclaration {
                name: "card".to_owned(),
                visibility: TransformTemplateModuleVisibility::Public,
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options,
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("runtime adapter should receive module options")
            .artifact;

        assert_eq!(compiled.opaque["moduleImports"], 1);
        assert_eq!(compiled.opaque["moduleEntrypoints"], 1);
        assert_eq!(compiled.entrypoint.name.as_deref(), Some("card"));
    }

    #[test]
    fn executable_template_adapter_wins_over_selector_only_builtin() {
        let mut registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
        registry.register(ExecutableCemNativeAdapter);
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };

        assert_eq!(
            registry.select(&identity),
            TransformTemplateAdapterResolution::Matched(TransformTemplateAdapterSelection {
                adapter_id: "executable-cem-native-template",
                kind: TransformTemplateKind::CemNative,
            })
        );
    }

    #[derive(Debug)]
    struct ExecutableCemNativeAdapter;

    impl TransformTemplateAdapter for ExecutableCemNativeAdapter {
        fn id(&self) -> &'static str {
            "executable-cem-native-template"
        }

        fn kind(&self) -> TransformTemplateKind {
            TransformTemplateKind::CemNative
        }

        fn capability(&self) -> TransformTemplateAdapterCapability {
            TransformTemplateAdapterCapability::Executable
        }

        fn matches_template(&self, identity: &FormatIdentity) -> bool {
            identity.content_type.as_deref() == Some("text/cem-ml")
        }
    }
}
