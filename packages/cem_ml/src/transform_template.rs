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
use crate::run_config::ScopeConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

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
    pub execution_policy: TransformExecutionPolicy,
}

pub const TRANSFORM_TEMPLATE_ENTRYPOINT_NOT_PUBLIC_CODE: &str =
    "cem.transform_template.entrypoint_not_public";
pub const TRANSFORM_TEMPLATE_PARAM_UNKNOWN_CODE: &str = "cem.transform_template.param_unknown";
pub const TRANSFORM_TEMPLATE_IMPORT_CYCLE_CODE: &str = "cem.transform_template.import_cycle";
pub const TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE: &str = "cem.transform_template.recursion_limit";
pub const TRANSFORM_TEMPLATE_INCLUDE_RESERVED_CODE: &str =
    "cem.transform_template.include_reserved";

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub visibility: TransformTemplateModuleVisibility,
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
    pub limits: TransformTemplateModuleLimits,
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
                "text/cem",
                "text/cem-ml",
            ],
            &[crate::schema::ir::CEM_CORE_NAMESPACE],
            &[crate::schema::ir::CEM_CORE_NAMESPACE],
        ));
        registry.register(StaticTransformTemplateAdapter::new(
            "xslt-template",
            TransformTemplateKind::Xslt,
            crate::legacy_custom_element::TEMPLATE_CONTENT_TYPES,
            &[],
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

    #[test]
    fn builtins_select_cem_native_and_xslt_template_adapters() {
        let registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
        let cem = FormatIdentity {
            content_type: Some("text/cem-ml; charset=utf-8".to_owned()),
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
            "defaultValue": "en-US"
        }))
        .expect("param defaults");
        let options = TransformTemplateModuleOptions {
            imports: vec![import],
            entrypoints: vec![entrypoint],
            params: vec![param],
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
        assert!(!options.params[0].required);
        assert_eq!(options.limits.max_import_depth, 32);
        assert_eq!(options.limits.max_recursion_depth, 64);
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
