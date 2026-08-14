//! Shared native query contracts and helpers over a built `CemDocument`.
//!
//! Tier A coverage per AC-Q-*: role / state lookup, validation message
//! traversal, label resolution via `id_table`, and source-map lookup that
//! traces any node back to its origin byte range.

use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::diagnostics::Diagnostic;
use crate::engine::FormatIdentity;
use crate::operation_control::{ExecutionScopeId, OperationControl};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::resolver::{ResolverPolicy, ResolverRegistry};
use crate::scheduler::{AbortSignal, ScopePolicy};
use crate::schema::registry::{
    content_type_essence, CEM_QL_CONTENT_TYPE, CEM_QL_EXPRESSION_CONTENT_TYPE,
    CEM_QL_EXPRESSION_SCHEMA_URI, CEM_QL_SCHEMA_URI, CSS_SELECTOR_CONTENT_TYPE,
    CSS_SELECTOR_SCHEMA_URI, XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI,
};
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack};

mod runtime;

pub use runtime::{
    query_execution_limits, run_query, select_query_language, QueryOwnedBindings,
    QueryPreparationRequest, QueryPreparedOwners, QueryRunContractError, QueryRunError,
    QueryRunFailure, QueryRunRequest, QueryRunResponse, QueryRuntimeAdapter, QueryRuntimeRegistry,
    QuerySource,
};

pub const CSS_SELECTOR_LANGUAGE_VERSION: &str = "selectors-4-20260122";
pub const CSS_SELECTOR_RESULT_REPRESENTATION_ID: &str = "cem.css-selector-result";

const CSS_SELECTOR_CONTENT_TYPES: &[&str] = &[CSS_SELECTOR_CONTENT_TYPE];
const CSS_SELECTOR_SCHEMA_URIS: &[&str] = &[CSS_SELECTOR_SCHEMA_URI];
const CEM_QL_QUERY_CONTENT_TYPES: &[&str] = &[
    CEM_QL_EXPRESSION_CONTENT_TYPE,
    CEM_QL_CONTENT_TYPE,
    "text/cem-ql",
];
const CEM_QL_QUERY_SCHEMA_URIS: &[&str] = &[CEM_QL_EXPRESSION_SCHEMA_URI, CEM_QL_SCHEMA_URI];
const XPATH_QUERY_CONTENT_TYPES: &[&str] = &[XPATH_CONTENT_TYPE, "text/xpath"];
const XPATH_QUERY_SCHEMA_URIS: &[&str] = &[XPATH_SCHEMA_URI];
const CSS_SELECTOR_INPUT_MODELS: &[QueryInputModel] = &[QueryInputModel::ElementTree];
const CEM_QL_INPUT_MODELS: &[QueryInputModel] = &[QueryInputModel::NativeItems];
const XPATH_INPUT_MODELS: &[QueryInputModel] = &[QueryInputModel::XdmTree];

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum QueryLanguage {
    CssSelector,
    CemQl,
    XPath,
}

impl QueryLanguage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CssSelector => "css-selector",
            Self::CemQl => "cem-ql",
            Self::XPath => "xpath",
        }
    }

    pub const fn contract(self) -> QueryLanguageContract {
        match self {
            Self::CssSelector => QueryLanguageContract {
                language: self,
                language_version: CSS_SELECTOR_LANGUAGE_VERSION,
                canonical_content_type: CSS_SELECTOR_CONTENT_TYPE,
                canonical_schema_uri: CSS_SELECTOR_SCHEMA_URI,
                accepted_content_types: CSS_SELECTOR_CONTENT_TYPES,
                accepted_schema_uris: CSS_SELECTOR_SCHEMA_URIS,
                input_models: CSS_SELECTOR_INPUT_MODELS,
                result_order: QueryResultOrder::DocumentOrder,
                duplicate_policy: QueryDuplicatePolicy::Eliminate,
                namespace_policy: QueryNamespacePolicy::ExplicitHostBindings,
            },
            Self::CemQl => QueryLanguageContract {
                language: self,
                language_version: "1.0.0",
                canonical_content_type: CEM_QL_EXPRESSION_CONTENT_TYPE,
                canonical_schema_uri: CEM_QL_EXPRESSION_SCHEMA_URI,
                accepted_content_types: CEM_QL_QUERY_CONTENT_TYPES,
                accepted_schema_uris: CEM_QL_QUERY_SCHEMA_URIS,
                input_models: CEM_QL_INPUT_MODELS,
                result_order: QueryResultOrder::LanguageDefined,
                duplicate_policy: QueryDuplicatePolicy::LanguageDefined,
                namespace_policy: QueryNamespacePolicy::LanguageStaticContext,
            },
            Self::XPath => QueryLanguageContract {
                language: self,
                language_version: "3.1",
                canonical_content_type: XPATH_CONTENT_TYPE,
                canonical_schema_uri: XPATH_SCHEMA_URI,
                accepted_content_types: XPATH_QUERY_CONTENT_TYPES,
                accepted_schema_uris: XPATH_QUERY_SCHEMA_URIS,
                input_models: XPATH_INPUT_MODELS,
                result_order: QueryResultOrder::LanguageDefined,
                duplicate_policy: QueryDuplicatePolicy::LanguageDefined,
                namespace_policy: QueryNamespacePolicy::LanguageStaticContext,
            },
        }
    }
}

impl fmt::Display for QueryLanguage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryInputModel {
    NativeItems,
    XdmTree,
    ElementTree,
}

impl QueryInputModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeItems => "native-items",
            Self::XdmTree => "xdm-tree",
            Self::ElementTree => "element-tree",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryResultOrder {
    LanguageDefined,
    DocumentOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryDuplicatePolicy {
    LanguageDefined,
    Eliminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryNamespacePolicy {
    LanguageStaticContext,
    ExplicitHostBindings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryLanguageContract {
    pub language: QueryLanguage,
    pub language_version: &'static str,
    pub canonical_content_type: &'static str,
    pub canonical_schema_uri: &'static str,
    pub accepted_content_types: &'static [&'static str],
    pub accepted_schema_uris: &'static [&'static str],
    pub input_models: &'static [QueryInputModel],
    pub result_order: QueryResultOrder,
    pub duplicate_policy: QueryDuplicatePolicy,
    pub namespace_policy: QueryNamespacePolicy,
}

impl QueryLanguageContract {
    pub fn matches_query_identity(self, identity: &FormatIdentity) -> bool {
        let content_type_matches = identity.content_type.as_deref().map(|content_type| {
            let essence = content_type_essence(content_type);
            self.accepted_content_types
                .iter()
                .any(|accepted| *accepted == essence)
        });
        let schema_matches = identity
            .schema
            .as_deref()
            .map(|schema| self.accepted_schema_uris.contains(&schema));
        match (content_type_matches, schema_matches) {
            (Some(content_type), Some(schema)) => content_type && schema,
            (Some(content_type), None) => content_type,
            (None, Some(schema)) => schema,
            (None, None) => false,
        }
    }

    pub fn supports_input_owner(self, owner: &dyn QueryInputOwner) -> bool {
        owner
            .input_models()
            .iter()
            .any(|model| self.input_models.contains(model))
    }
}

pub trait QueryNativeArtifact: Any + Send + Sync {
    fn representation_id(&self) -> &'static str;
    fn source_map(&self) -> Option<&SourceMapStack>;
    fn as_any(&self) -> &dyn Any;
}

pub trait QueryAstOwner: QueryNativeArtifact {
    fn language(&self) -> QueryLanguage;
    fn identity(&self) -> &FormatIdentity;
    fn source_uri(&self) -> &str;
}

pub trait QueryInputOwner: QueryNativeArtifact {
    fn identity(&self) -> &FormatIdentity;
    fn input_models(&self) -> &[QueryInputModel];
}

pub trait QueryNativeResult: QueryNativeArtifact {
    fn language(&self) -> QueryLanguage;
}

pub type QueryNativeBindings<'a> = BTreeMap<String, &'a dyn QueryNativeArtifact>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueryExecutionLimits {
    pub max_result_items: Option<u64>,
    pub max_work_units: Option<u64>,
}

pub struct QueryExecutionRequest<'a> {
    pub language: QueryLanguage,
    pub query_ast_owner: &'a dyn QueryAstOwner,
    pub input_ast_owner: &'a dyn QueryInputOwner,
    pub context_item: Option<&'a dyn QueryNativeArtifact>,
    pub bindings: &'a QueryNativeBindings<'a>,
    pub namespace_bindings: &'a BTreeMap<String, String>,
    pub resolver_registry: &'a ResolverRegistry,
    pub resolver_policy: &'a ResolverPolicy,
    pub resolver_policy_stamp: &'a str,
    pub safety_policy_stamp: &'a str,
    pub scope_policy: &'a ScopePolicy,
    pub operation_control: &'a OperationControl,
    pub execution_scope: ExecutionScopeId,
    pub abort_signal: &'a AbortSignal,
    pub limits: QueryExecutionLimits,
}

impl QueryExecutionRequest<'_> {
    pub fn validate_contract(&self) -> Result<(), QueryContractError> {
        if self.query_ast_owner.language() != self.language {
            return Err(QueryContractError::QueryLanguageMismatch {
                requested: self.language,
                owner: self.query_ast_owner.language(),
            });
        }
        let contract = self.language.contract();
        if !contract.matches_query_identity(self.query_ast_owner.identity()) {
            return Err(QueryContractError::QueryIdentityUnsupported {
                language: self.language,
            });
        }
        if !contract.supports_input_owner(self.input_ast_owner) {
            return Err(QueryContractError::InputModelUnsupported {
                language: self.language,
                actual: self.input_ast_owner.input_models().to_vec(),
            });
        }
        if self.resolver_policy_stamp.trim().is_empty() {
            return Err(QueryContractError::ResolverPolicyStampMissing);
        }
        if self.safety_policy_stamp.trim().is_empty() {
            return Err(QueryContractError::SafetyPolicyStampMissing);
        }
        self.scope_policy
            .validate()
            .map_err(|error| QueryContractError::ScopePolicyInvalid {
                message: error.to_string(),
            })?;
        if self
            .operation_control
            .check_scope(self.execution_scope)
            .is_err()
            || self.abort_signal.is_aborted()
        {
            return Err(QueryContractError::Aborted);
        }
        if let Some((prefix, _)) = self
            .namespace_bindings
            .iter()
            .find(|(_, namespace_uri)| namespace_uri.trim().is_empty())
        {
            return Err(QueryContractError::NamespaceUriEmpty {
                prefix: prefix.clone(),
            });
        }
        if let Some(name) = self.bindings.keys().find(|name| name.trim().is_empty()) {
            return Err(QueryContractError::BindingNameEmpty { name: name.clone() });
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct QueryExecutionResult {
    pub language: QueryLanguage,
    pub query_identity: FormatIdentity,
    pub input_ast_owner: Arc<dyn QueryInputOwner>,
    pub native_result: Arc<dyn QueryNativeResult>,
    pub source_map: SourceMapStack,
}

impl fmt::Debug for QueryExecutionResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryExecutionResult")
            .field("language", &self.language)
            .field("query_identity", &self.query_identity)
            .field("input_ast_owner", &self.input_ast_owner.representation_id())
            .field("native_result", &self.native_result.representation_id())
            .field("source_map", &self.source_map)
            .finish()
    }
}

impl QueryExecutionResult {
    pub fn new(
        language: QueryLanguage,
        query_identity: FormatIdentity,
        input_ast_owner: Arc<dyn QueryInputOwner>,
        native_result: Arc<dyn QueryNativeResult>,
        source_map: SourceMapStack,
    ) -> Result<Self, QueryContractError> {
        let contract = language.contract();
        if !contract.matches_query_identity(&query_identity) {
            return Err(QueryContractError::QueryIdentityUnsupported { language });
        }
        if !contract.supports_input_owner(input_ast_owner.as_ref()) {
            return Err(QueryContractError::InputModelUnsupported {
                language,
                actual: input_ast_owner.input_models().to_vec(),
            });
        }
        if native_result.language() != language {
            return Err(QueryContractError::ResultLanguageMismatch {
                expected: language,
                actual: native_result.language(),
            });
        }
        Ok(Self {
            language,
            query_identity,
            input_ast_owner,
            native_result,
            source_map,
        })
    }
}

pub trait QueryEvaluatorAdapter: Send + Sync {
    fn language(&self) -> QueryLanguage;
    fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>>;
}

#[derive(Default)]
pub struct QueryEvaluatorRegistry {
    evaluators: BTreeMap<QueryLanguage, Arc<dyn QueryEvaluatorAdapter>>,
}

impl QueryEvaluatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, evaluator: impl QueryEvaluatorAdapter + 'static) {
        self.evaluators
            .insert(evaluator.language(), Arc::new(evaluator));
    }

    pub fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>> {
        let language = request.language;
        self.evaluators
            .get(&language)
            .ok_or_else(|| {
                vec![Diagnostic {
                    code: "cem.query.evaluator_unavailable".to_owned(),
                    severity: crate::diagnostics::Severity::Fatal,
                    message: format!("no native query evaluator is registered for `{language}`"),
                    ..Diagnostic::default()
                }]
            })?
            .evaluate(request)
    }
}

impl fmt::Debug for QueryEvaluatorRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryEvaluatorRegistry")
            .field("languages", &self.evaluators.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum QueryExportFormat {
    Terminal,
    Cem,
    Json,
}

impl QueryExportFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Cem => "cem",
            Self::Json => "json",
        }
    }
}

pub struct QueryExportRequest<'a> {
    pub result: &'a QueryExecutionResult,
    pub no_color: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryEncodedOutput {
    pub content_type: String,
    pub bytes: Vec<u8>,
}

pub trait QueryResultExporter: Send + Sync {
    fn id(&self) -> &'static str;
    fn language(&self) -> QueryLanguage;
    fn format(&self) -> QueryExportFormat;
    fn export(&self, request: QueryExportRequest<'_>) -> Result<QueryEncodedOutput, String>;
}

#[derive(Default)]
pub struct QueryResultExporterRegistry {
    exporters: BTreeMap<(QueryLanguage, QueryExportFormat), Arc<dyn QueryResultExporter>>,
}

impl QueryResultExporterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, exporter: impl QueryResultExporter + 'static) {
        self.exporters
            .insert((exporter.language(), exporter.format()), Arc::new(exporter));
    }

    pub fn contains(&self, language: QueryLanguage, format: QueryExportFormat) -> bool {
        self.exporters.contains_key(&(language, format))
    }

    pub fn export(
        &self,
        format: QueryExportFormat,
        request: QueryExportRequest<'_>,
    ) -> Result<QueryEncodedOutput, String> {
        let language = request.result.language;
        let exporter = self.exporters.get(&(language, format)).ok_or_else(|| {
            format!(
                "no query result exporter is registered for `{language}` and `{}`",
                format.as_str()
            )
        })?;
        exporter.export(request).map_err(|message| {
            format!(
                "query result exporter `{}` failed for `{language}` and `{}`: {message}",
                exporter.id(),
                format.as_str()
            )
        })
    }
}

impl fmt::Debug for QueryResultExporterRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryResultExporterRegistry")
            .field("boundaries", &self.exporters.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryContractError {
    QueryLanguageMismatch {
        requested: QueryLanguage,
        owner: QueryLanguage,
    },
    QueryIdentityUnsupported {
        language: QueryLanguage,
    },
    InputModelUnsupported {
        language: QueryLanguage,
        actual: Vec<QueryInputModel>,
    },
    ResultLanguageMismatch {
        expected: QueryLanguage,
        actual: QueryLanguage,
    },
    ResolverPolicyStampMissing,
    SafetyPolicyStampMissing,
    ScopePolicyInvalid {
        message: String,
    },
    NamespaceUriEmpty {
        prefix: String,
    },
    BindingNameEmpty {
        name: String,
    },
    Aborted,
}

impl fmt::Display for QueryContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueryLanguageMismatch { requested, owner } => write!(
                formatter,
                "query language `{requested}` does not match AST owner language `{owner}`"
            ),
            Self::QueryIdentityUnsupported { language } => {
                write!(formatter, "query identity is not owned by `{language}`")
            }
            Self::InputModelUnsupported { language, actual } => write!(
                formatter,
                "query language `{language}` cannot consume input models {actual:?}"
            ),
            Self::ResultLanguageMismatch { expected, actual } => write!(
                formatter,
                "query result language `{actual}` does not match `{expected}`"
            ),
            Self::ResolverPolicyStampMissing => {
                formatter.write_str("query resolver policy stamp is required")
            }
            Self::SafetyPolicyStampMissing => {
                formatter.write_str("query safety policy stamp is required")
            }
            Self::ScopePolicyInvalid { message } => {
                write!(formatter, "query scope policy is invalid: {message}")
            }
            Self::NamespaceUriEmpty { prefix } => {
                write!(formatter, "query namespace `{prefix}` has an empty URI")
            }
            Self::BindingNameEmpty { name } => {
                write!(formatter, "query binding name `{name}` is empty")
            }
            Self::Aborted => formatter.write_str("query execution was aborted"),
        }
    }
}

impl std::error::Error for QueryContractError {}

/// Return the element node id whose `id="..."` attribute matched `target`,
/// or `None` if no element registered that id.
pub fn find_by_id<'a>(doc: &'a CemDocument, target: &str) -> Option<&'a CemAstNode> {
    let id = doc.id_table.get(target)?;
    doc.get(*id)
}

/// Iterator over every element node in document order.
pub fn elements(doc: &CemDocument) -> impl Iterator<Item = &CemAstNode> {
    doc.iter()
        .filter(|n| matches!(n, CemAstNode::Element { .. }))
}

/// Element node ids whose lexical local name (after `:` if present)
/// matches `local`.
pub fn find_by_local_name<'a>(
    doc: &'a CemDocument,
    local: &'a str,
) -> impl Iterator<Item = &'a CemAstNode> {
    doc.iter().filter(move |n| match n {
        CemAstNode::Element { expanded_name, .. } => expanded_name.local_name == local,
        _ => false,
    })
}

/// Every attribute on an element whose name carries the given namespace
/// prefix (e.g. `"cem"`). Tier A AST stores the lexical prefix in
/// `expanded_name.namespace_uri` until full namespace expansion lands;
/// `prefix = ""` selects unprefixed attributes.
pub fn attributes_in_prefix<'a>(
    doc: &'a CemDocument,
    element: &'a CemAstNode,
    prefix: &'a str,
) -> impl Iterator<Item = &'a CemAstNode> {
    let attr_ids: &[AstNodeId] = match element {
        CemAstNode::Element { attributes, .. } => attributes,
        _ => &[],
    };
    attr_ids.iter().filter_map(move |id| {
        let node = doc.get(*id)?;
        if let CemAstNode::Attribute { expanded_name, .. } = node {
            if expanded_name.namespace_uri == prefix {
                return Some(node);
            }
        }
        None
    })
}

/// CEM annotations on an element: attributes in the `cem:` namespace
/// excluding the `cem:state` attribute.
pub fn cem_annotations<'a>(
    doc: &'a CemDocument,
    element: &'a CemAstNode,
) -> impl Iterator<Item = &'a CemAstNode> {
    attributes_in_prefix(doc, element, "cem").filter(|attr| match attr {
        CemAstNode::Attribute { expanded_name, .. } => expanded_name.local_name != "state",
        _ => false,
    })
}

/// Element node ids that carry the CEM annotation with the given local
/// name (e.g. `"screen"`, `"action"`).
pub fn elements_with_annotation<'a>(
    doc: &'a CemDocument,
    annotation_local: &'a str,
) -> impl Iterator<Item = &'a CemAstNode> {
    doc.iter().filter(move |node| {
        let CemAstNode::Element { attributes, .. } = node else {
            return false;
        };
        attributes.iter().any(|attr_id| match doc.get(*attr_id) {
            Some(CemAstNode::Attribute { expanded_name, .. }) => {
                expanded_name.namespace_uri == "cem" && expanded_name.local_name == annotation_local
            }
            _ => false,
        })
    })
}

/// Decoded state names attached to an element via `cem:state="..."`.
/// Returns an empty `Vec` if the element has no state attribute.
pub fn state_of(doc: &CemDocument, element: &CemAstNode) -> Vec<String> {
    let CemAstNode::Element { attributes, .. } = element else {
        return Vec::new();
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
        if expanded_name.namespace_uri == "cem" && expanded_name.local_name == "state" {
            return value
                .as_deref()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect();
        }
    }
    Vec::new()
}

/// Project a node back to its origin byte range, walking the source-map
/// stack origin-first.
pub fn origin_byte_range(node: &CemAstNode) -> Option<ByteRange> {
    let stack = match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    };
    stack.frames.first().and_then(|frame| match &frame.span {
        FrameSpan::Single(r) => Some(*r),
        FrameSpan::Multi(rs) => rs.first().copied(),
    })
}

/// Validation diagnostics on this document. Equivalent to `doc.diagnostics`,
/// kept as a function so consumers can compose with other queries.
pub fn validation_messages(doc: &CemDocument) -> &[Diagnostic] {
    &doc.diagnostics
}

/// Resolve a `for`/`aria-*` reference attribute's value through the
/// document `id_table`. Returns the resolved target node or `None` if the
/// reference is unresolved (which the AST builder already recorded as a
/// `cem.ast.unresolved_reference` diagnostic).
pub fn resolve_reference<'a>(
    doc: &'a CemDocument,
    attribute: &CemAstNode,
) -> Option<&'a CemAstNode> {
    let value = match attribute {
        CemAstNode::Attribute { value, .. } => value.as_deref()?,
        _ => return None,
    };
    find_by_id(doc, value)
}

/// Walk every source-map frame on a node from origin to current.
pub fn source_map_frames(node: &CemAstNode) -> &[SourceMapFrame] {
    let stack = match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    };
    &stack.frames
}

#[cfg(test)]
mod query_execution_contract_tests {
    use std::any::Any;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::*;
    use crate::engine::FormatIdentity;
    use crate::resolver::{ResolverPolicy, ResolverRegistry};
    use crate::run_config::ScopeConfig;
    use crate::scheduler::{AbortSignal, ScopePolicy};
    use crate::schema::registry::{
        CSS_CONTENT_TYPE, CSS_SCHEMA_URI, CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI,
        XML_CONTENT_TYPE, XML_SCHEMA_URI, XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI,
    };
    use crate::source_map::SourceMapStack;

    #[derive(Debug)]
    struct TestNativeArtifact {
        representation_id: &'static str,
        source_map: SourceMapStack,
    }

    impl QueryNativeArtifact for TestNativeArtifact {
        fn representation_id(&self) -> &'static str {
            self.representation_id
        }

        fn source_map(&self) -> Option<&SourceMapStack> {
            Some(&self.source_map)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct TestQueryAst {
        native: TestNativeArtifact,
        language: QueryLanguage,
        identity: FormatIdentity,
        source_uri: String,
    }

    impl QueryNativeArtifact for TestQueryAst {
        fn representation_id(&self) -> &'static str {
            self.native.representation_id()
        }

        fn source_map(&self) -> Option<&SourceMapStack> {
            self.native.source_map()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl QueryAstOwner for TestQueryAst {
        fn language(&self) -> QueryLanguage {
            self.language
        }

        fn identity(&self) -> &FormatIdentity {
            &self.identity
        }

        fn source_uri(&self) -> &str {
            &self.source_uri
        }
    }

    #[derive(Debug)]
    struct TestInputAst {
        native: TestNativeArtifact,
        identity: FormatIdentity,
        input_models: Vec<QueryInputModel>,
    }

    impl QueryNativeArtifact for TestInputAst {
        fn representation_id(&self) -> &'static str {
            self.native.representation_id()
        }

        fn source_map(&self) -> Option<&SourceMapStack> {
            self.native.source_map()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl QueryInputOwner for TestInputAst {
        fn identity(&self) -> &FormatIdentity {
            &self.identity
        }

        fn input_models(&self) -> &[QueryInputModel] {
            &self.input_models
        }
    }

    #[derive(Debug)]
    struct TestQueryResult(TestNativeArtifact);

    impl QueryNativeArtifact for TestQueryResult {
        fn representation_id(&self) -> &'static str {
            self.0.representation_id()
        }

        fn source_map(&self) -> Option<&SourceMapStack> {
            self.0.source_map()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl QueryNativeResult for TestQueryResult {
        fn language(&self) -> QueryLanguage {
            QueryLanguage::CssSelector
        }
    }

    fn identity(content_type: &str, schema: &str) -> FormatIdentity {
        FormatIdentity {
            content_type: Some(content_type.to_owned()),
            schema: Some(schema.to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn query_run_request(
        language: QueryLanguage,
        query_bytes: &[u8],
        query_identity: FormatIdentity,
    ) -> QueryRunRequest {
        let input_identity = identity(XML_CONTENT_TYPE, XML_SCHEMA_URI);
        QueryRunRequest {
            data: crate::engine::EngineInput {
                uri: "memory:catalog.xml".to_owned(),
                bytes: b"<catalog><book id=\"a\"/><book id=\"b\"/></catalog>".to_vec(),
                from_format: Some(crate::engine::InputFormat::Xml),
                identity: Some(input_identity.clone()),
                root_scope: ScopeConfig {
                    default_content_type: input_identity.content_type.clone(),
                    schema: input_identity.schema.clone(),
                    ..ScopeConfig::default()
                },
            },
            query: QuerySource {
                uri: format!("memory:query.{}", language.as_str()),
                bytes: query_bytes.to_vec(),
                identity: query_identity,
            },
            context: crate::engine::EngineContext::default(),
            context_item: None,
            bindings: QueryOwnedBindings::new(),
            limits: None,
        }
    }

    #[test]
    fn css_selector_identity_is_distinct_from_stylesheet_css() {
        let contract = QueryLanguage::CssSelector.contract();
        assert_eq!(contract.canonical_content_type, CSS_SELECTOR_CONTENT_TYPE);
        assert_eq!(contract.canonical_schema_uri, CSS_SELECTOR_SCHEMA_URI);
        assert!(contract.matches_query_identity(&identity(
            CSS_SELECTOR_CONTENT_TYPE,
            CSS_SELECTOR_SCHEMA_URI,
        )));
        assert!(!contract.matches_query_identity(&identity(CSS_CONTENT_TYPE, CSS_SCHEMA_URI)));
    }

    #[test]
    fn builtin_query_contracts_pin_native_input_and_result_semantics() {
        let css = QueryLanguage::CssSelector.contract();
        assert_eq!(css.input_models, &[QueryInputModel::ElementTree]);
        assert_eq!(css.result_order, QueryResultOrder::DocumentOrder);
        assert_eq!(css.duplicate_policy, QueryDuplicatePolicy::Eliminate);
        assert_eq!(
            css.namespace_policy,
            QueryNamespacePolicy::ExplicitHostBindings
        );

        let cem_ql = QueryLanguage::CemQl.contract();
        assert_eq!(cem_ql.input_models, &[QueryInputModel::NativeItems]);
        assert_eq!(cem_ql.result_order, QueryResultOrder::LanguageDefined);

        let xpath = QueryLanguage::XPath.contract();
        assert_eq!(xpath.input_models, &[QueryInputModel::XdmTree]);
        assert_eq!(xpath.result_order, QueryResultOrder::LanguageDefined);
    }

    #[test]
    fn shared_request_and_result_retain_native_owners_and_policies() {
        let query = TestQueryAst {
            native: TestNativeArtifact {
                representation_id: "test.css-selector-ast",
                source_map: SourceMapStack::default(),
            },
            language: QueryLanguage::CssSelector,
            identity: identity(CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI),
            source_uri: "memory:query.css-selector".to_owned(),
        };
        let input = Arc::new(TestInputAst {
            native: TestNativeArtifact {
                representation_id: "test.element-tree",
                source_map: SourceMapStack::default(),
            },
            identity: identity("text/html", "https://cem.dev/ns/data/html/1"),
            input_models: vec![QueryInputModel::ElementTree],
        });
        let binding = TestNativeArtifact {
            representation_id: "test.binding",
            source_map: SourceMapStack::default(),
        };
        let bindings: QueryNativeBindings<'_> =
            BTreeMap::from([("limit".to_owned(), &binding as &dyn QueryNativeArtifact)]);
        let namespaces =
            BTreeMap::from([("svg".to_owned(), "http://www.w3.org/2000/svg".to_owned())]);
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let scope_policy = ScopePolicy::host_root();
        let abort_signal = AbortSignal::new();
        let operation_control = OperationControl::new(abort_signal.clone());

        let request = QueryExecutionRequest {
            language: QueryLanguage::CssSelector,
            query_ast_owner: &query,
            input_ast_owner: input.as_ref(),
            context_item: None,
            bindings: &bindings,
            namespace_bindings: &namespaces,
            resolver_registry: &resolver_registry,
            resolver_policy: &resolver_policy,
            resolver_policy_stamp: "resolver-policy/1",
            safety_policy_stamp: "query-safety/1",
            scope_policy: &scope_policy,
            operation_control: &operation_control,
            execution_scope: crate::operation_control::ROOT_EXECUTION_SCOPE_ID,
            abort_signal: &abort_signal,
            limits: QueryExecutionLimits {
                max_result_items: Some(10),
                max_work_units: Some(100),
            },
        };
        request
            .validate_contract()
            .expect("native CSS selector request should satisfy the shared contract");
        assert!(request
            .query_ast_owner
            .as_any()
            .downcast_ref::<TestQueryAst>()
            .is_some());
        assert!(request
            .input_ast_owner
            .as_any()
            .downcast_ref::<TestInputAst>()
            .is_some());

        let unsupported_input = TestInputAst {
            native: TestNativeArtifact {
                representation_id: "test.native-items",
                source_map: SourceMapStack::default(),
            },
            identity: identity("application/json", "https://cem.dev/ns/data/json/1"),
            input_models: vec![QueryInputModel::NativeItems],
        };
        let unsupported_request = QueryExecutionRequest {
            input_ast_owner: &unsupported_input,
            ..request
        };
        assert!(matches!(
            unsupported_request.validate_contract(),
            Err(QueryContractError::InputModelUnsupported {
                language: QueryLanguage::CssSelector,
                actual,
            }) if actual == vec![QueryInputModel::NativeItems]
        ));

        let input_owner: Arc<dyn QueryInputOwner> = input;
        let native_result: Arc<dyn QueryNativeResult> =
            Arc::new(TestQueryResult(TestNativeArtifact {
                representation_id: "test.css-selector-result",
                source_map: SourceMapStack::default(),
            }));
        let result = QueryExecutionResult::new(
            QueryLanguage::CssSelector,
            query.identity.clone(),
            input_owner,
            native_result,
            SourceMapStack::default(),
        )
        .expect("native result should retain matching owners");
        assert_eq!(
            result.input_ast_owner.representation_id(),
            "test.element-tree"
        );
        assert_eq!(
            result.native_result.representation_id(),
            "test.css-selector-result"
        );
    }

    #[test]
    fn high_level_query_run_owns_sources_and_native_results_for_builtin_languages() {
        for (language, query_bytes, query_identity, expected_result) in [
            (
                QueryLanguage::CssSelector,
                b"book".as_slice(),
                identity(CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI),
                CSS_SELECTOR_RESULT_REPRESENTATION_ID,
            ),
            (
                QueryLanguage::XPath,
                b"/catalog/book".as_slice(),
                identity(XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI),
                crate::transform_artifact::XPATH_RESULT_REPRESENTATION_ID,
            ),
        ] {
            let request = query_run_request(language, query_bytes, query_identity);

            let response = run_query(request).expect("builtin query run succeeds");
            assert_eq!(response.language, language);
            assert_eq!(
                response.inputs,
                [
                    "memory:catalog.xml".to_owned(),
                    format!("memory:query.{}", language.as_str()),
                ]
            );
            assert_eq!(
                response.result.native_result.representation_id(),
                expected_result
            );
            assert!(!response.result.source_map.frames.is_empty());
            assert!(response
                .diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
        }
    }

    #[test]
    fn high_level_query_run_preserves_budget_and_host_abort_contracts() {
        let mut invalid_budget = query_run_request(
            QueryLanguage::CssSelector,
            b"book",
            identity(CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI),
        );
        invalid_budget
            .data
            .root_scope
            .budgets
            .insert("queryItems".to_owned(), "many".to_owned());
        assert!(matches!(
            run_query(invalid_budget),
            Err(QueryRunError::Contract(QueryRunContractError::BudgetInvalid { name }))
                if name == "queryItems"
        ));

        let aborted = query_run_request(
            QueryLanguage::CssSelector,
            b"book",
            identity(CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI),
        );
        aborted.context.abort_signal().abort();
        let Err(QueryRunError::Execution(failure)) = run_query(aborted) else {
            panic!("pre-aborted query must return a typed execution failure");
        };
        assert_eq!(
            failure.inputs,
            [
                "memory:catalog.xml".to_owned(),
                "memory:query.css-selector".to_owned(),
            ]
        );
        assert!(failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("aborted")));
    }

    #[test]
    fn cli_query_dispatch_delegates_native_orchestration_to_the_common_runner() {
        let dispatch = include_str!("../../cem_ml_cli/src/dispatch.rs");
        let query_dispatch = dispatch
            .split("pub fn run_query")
            .nth(1)
            .and_then(|source| source.split("fn run_transform_graph").next())
            .expect("CLI query dispatch source");

        assert!(query_dispatch.contains("cem_ml::query::run_query(QueryRunRequest"));
        for engine_owned_symbol in [
            "LifecycleRegistry::with_builtin_adapters",
            "CssSelectorElementTreeOwner",
            "CemQlNativeItemsOwner",
            "XPathXdmTreeOwner",
            "CemQlQueryAstOwner",
            "QueryEvaluatorRegistry",
            "QueryExecutionRequest",
        ] {
            assert!(
                !query_dispatch.contains(engine_owned_symbol),
                "CLI query dispatch must not recover engine-owned `{engine_owned_symbol}` orchestration"
            );
        }
        assert!(query_dispatch.contains("QueryResultExporterRegistry"));
        assert!(query_dispatch.contains("write_query_report"));
        assert!(query_dispatch.contains("write_destination"));
    }
}
