//! Owned high-level query orchestration over native lifecycle and language adapters.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{EngineContext, EngineInput, FormatIdentity};
use crate::lifecycle::{LifecycleRegistry, LoadedInputAstStream};
use crate::run_config::ScopeConfig;
use crate::scheduler::ScopePolicy;
use crate::validation::css_selector::{
    css_selector_expression_ast_from_source_bytes, CemCssSelectorEvaluator,
    CssSelectorElementTreeOwner, CssSelectorSourceRequest,
};
use crate::validation::xpath::{
    CemXPathQueryEvaluator, XPathQueryAstOwner, XPathSourceRequest, XPathXdmTreeOwner,
};

use super::{
    QueryAstOwner, QueryEvaluatorAdapter, QueryExecutionLimits, QueryExecutionRequest,
    QueryExecutionResult, QueryInputOwner, QueryLanguage, QueryNativeArtifact, QueryNativeBindings,
};

pub type QueryOwnedBindings = BTreeMap<String, Arc<dyn QueryNativeArtifact>>;

#[derive(Debug, Clone)]
pub struct QuerySource {
    pub uri: String,
    pub bytes: Vec<u8>,
    pub identity: FormatIdentity,
}

#[derive(Clone)]
pub struct QueryRunRequest {
    pub data: EngineInput,
    pub query: QuerySource,
    pub context: EngineContext,
    pub context_item: Option<Arc<dyn QueryNativeArtifact>>,
    pub bindings: QueryOwnedBindings,
    pub limits: Option<QueryExecutionLimits>,
}

impl fmt::Debug for QueryRunRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryRunRequest")
            .field("data", &self.data)
            .field("query", &self.query)
            .field("context", &self.context)
            .field(
                "context_item",
                &self
                    .context_item
                    .as_ref()
                    .map(|item| item.representation_id()),
            )
            .field("bindings", &self.bindings.keys().collect::<Vec<_>>())
            .field("limits", &self.limits)
            .field("operation_control", &self.context.operation_control)
            .finish()
    }
}

#[derive(Clone)]
pub struct QueryRunResponse {
    pub language: QueryLanguage,
    pub inputs: [String; 2],
    pub result: QueryExecutionResult,
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Debug for QueryRunResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryRunResponse")
            .field("language", &self.language)
            .field("inputs", &self.inputs)
            .field("result", &self.result)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct QueryRunFailure {
    pub language: QueryLanguage,
    pub inputs: [String; 2],
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryRunContractError {
    IdentityUnsupported {
        content_type: Option<String>,
        schema: Option<String>,
    },
    IdentityAmbiguous {
        languages: Vec<QueryLanguage>,
    },
    BudgetInvalid {
        name: String,
    },
}

impl fmt::Display for QueryRunContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityUnsupported {
                content_type,
                schema,
            } => write!(
                formatter,
                "query identity did not match CSS selector, CEM-QL, or XPath: content type `{}` schema `{}`",
                content_type.as_deref().unwrap_or("none"),
                schema.as_deref().unwrap_or("none")
            ),
            Self::IdentityAmbiguous { languages } => write!(
                formatter,
                "query identity is ambiguous across registered languages: {}",
                languages
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::BudgetInvalid { name } => write!(
                formatter,
                "query scope budget `{name}` must be a non-negative integer"
            ),
        }
    }
}

impl std::error::Error for QueryRunContractError {}

#[derive(Debug, Clone)]
pub enum QueryRunError {
    Contract(QueryRunContractError),
    Execution(QueryRunFailure),
}

impl fmt::Display for QueryRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => error.fmt(formatter),
            Self::Execution(failure) => write!(
                formatter,
                "query execution failed with {} diagnostic(s)",
                failure.diagnostics.len()
            ),
        }
    }
}

impl std::error::Error for QueryRunError {}

pub struct QueryPreparationRequest<'a> {
    pub input_uri: &'a str,
    pub input_identity: FormatIdentity,
    pub lifecycle_owner: Arc<LoadedInputAstStream>,
    pub query: &'a QuerySource,
    pub resolver_policy_stamp: &'a str,
}

pub struct QueryPreparedOwners {
    pub query_ast_owner: Arc<dyn QueryAstOwner>,
    pub input_ast_owner: Arc<dyn QueryInputOwner>,
    pub diagnostics: Vec<Diagnostic>,
}

pub trait QueryRuntimeAdapter: Send + Sync {
    fn language(&self) -> QueryLanguage;

    fn prepare(
        &self,
        request: QueryPreparationRequest<'_>,
    ) -> Result<QueryPreparedOwners, Vec<Diagnostic>>;

    fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>>;
}

#[derive(Clone, Default)]
pub struct QueryRuntimeRegistry {
    adapters: BTreeMap<QueryLanguage, Arc<dyn QueryRuntimeAdapter>>,
}

impl QueryRuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_adapters() -> Self {
        let mut registry = Self::new();
        registry.register(CssSelectorQueryRuntimeAdapter);
        registry.register(XPathQueryRuntimeAdapter);
        registry
    }

    pub fn register(&mut self, adapter: impl QueryRuntimeAdapter + 'static) {
        self.adapters.insert(adapter.language(), Arc::new(adapter));
    }

    fn adapter(&self, language: QueryLanguage) -> Option<Arc<dyn QueryRuntimeAdapter>> {
        self.adapters.get(&language).cloned()
    }
}

impl fmt::Debug for QueryRuntimeRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryRuntimeRegistry")
            .field("languages", &self.adapters.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
struct CssSelectorQueryRuntimeAdapter;

impl QueryRuntimeAdapter for CssSelectorQueryRuntimeAdapter {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CssSelector
    }

    fn prepare(
        &self,
        request: QueryPreparationRequest<'_>,
    ) -> Result<QueryPreparedOwners, Vec<Diagnostic>> {
        let input = CssSelectorElementTreeOwner::from_lifecycle(
            request.lifecycle_owner,
            request.input_identity,
        )
        .map_err(|fact| {
            vec![fatal_diagnostic(
                request.input_uri,
                "cem.css_selector.input_unsupported",
                fact.message,
            )]
        })?;
        let (query, diagnostics) =
            css_selector_expression_ast_from_source_bytes(CssSelectorSourceRequest {
                bytes: &request.query.bytes,
                source_uri: &request.query.uri,
                content_type: request.query.identity.content_type.as_deref(),
                namespace_bindings: &request.query.identity.namespaces,
            });
        let query = query.ok_or_else(|| diagnostics.clone())?;
        Ok(QueryPreparedOwners {
            query_ast_owner: Arc::new(query),
            input_ast_owner: Arc::new(input),
            diagnostics,
        })
    }

    fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>> {
        CemCssSelectorEvaluator.evaluate(request)
    }
}

#[derive(Debug, Clone, Copy)]
struct XPathQueryRuntimeAdapter;

impl QueryRuntimeAdapter for XPathQueryRuntimeAdapter {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::XPath
    }

    fn prepare(
        &self,
        request: QueryPreparationRequest<'_>,
    ) -> Result<QueryPreparedOwners, Vec<Diagnostic>> {
        let input =
            XPathXdmTreeOwner::from_lifecycle(request.lifecycle_owner, request.input_identity)
                .map_err(|message| {
                    vec![fatal_diagnostic(
                        request.input_uri,
                        "cem.xpath.query_input_unsupported",
                        message,
                    )]
                })?;
        let (query, diagnostics) = XPathQueryAstOwner::from_source_bytes(
            XPathSourceRequest {
                bytes: &request.query.bytes,
                source_uri: &request.query.uri,
                content_type: request.query.identity.content_type.as_deref(),
                source_range_projector: None,
            },
            request.query.identity.clone(),
        );
        Ok(QueryPreparedOwners {
            query_ast_owner: Arc::new(query),
            input_ast_owner: Arc::new(input),
            diagnostics,
        })
    }

    fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>> {
        CemXPathQueryEvaluator.evaluate(request)
    }
}

pub fn select_query_language(
    identity: &FormatIdentity,
) -> Result<QueryLanguage, QueryRunContractError> {
    let matches = [
        QueryLanguage::CssSelector,
        QueryLanguage::CemQl,
        QueryLanguage::XPath,
    ]
    .into_iter()
    .filter(|language| language.contract().matches_query_identity(identity))
    .collect::<Vec<_>>();
    match matches.as_slice() {
        [language] => Ok(*language),
        [] => Err(QueryRunContractError::IdentityUnsupported {
            content_type: identity.content_type.clone(),
            schema: identity.schema.clone(),
        }),
        languages => Err(QueryRunContractError::IdentityAmbiguous {
            languages: languages.to_vec(),
        }),
    }
}

pub fn query_execution_limits(
    language: QueryLanguage,
    scope: &ScopeConfig,
) -> Result<QueryExecutionLimits, QueryRunContractError> {
    let result_names: &[&str] = if language == QueryLanguage::XPath {
        &["queryitems", "xpathitems"]
    } else {
        &["queryitems"]
    };
    Ok(QueryExecutionLimits {
        max_result_items: query_budget_value(scope, result_names)?,
        max_work_units: query_budget_value(scope, &["querywork"])?,
    })
}

pub fn run_query(request: QueryRunRequest) -> Result<QueryRunResponse, QueryRunError> {
    let language =
        select_query_language(&request.query.identity).map_err(QueryRunError::Contract)?;
    let inputs = [request.data.uri.clone(), request.query.uri.clone()];
    if request.context.abort_signal().is_aborted() {
        return Err(execution_failure(
            language,
            inputs,
            vec![query_cancelled_diagnostic(
                &request.data.uri,
                request.context.abort_signal(),
            )],
        ));
    }
    let input_identity = request
        .data
        .identity
        .clone()
        .or_else(|| request.data.root_scope.format_identity_option())
        .unwrap_or_else(|| FormatIdentity::from(&request.context));
    let limits = request
        .limits
        .map(Ok)
        .unwrap_or_else(|| query_execution_limits(language, &request.data.root_scope))
        .map_err(QueryRunError::Contract)?;
    let resolver_policy_stamp = request.context.resolver_policy.cache_stamp();
    let scope_policy = query_scope_policy(&request.context);
    let safety_policy_stamp = query_safety_policy_stamp(limits);

    let mut loaded =
        LifecycleRegistry::with_builtin_adapters().load(&request.data, &request.context);
    let mut diagnostics = loaded.diagnostics;
    if has_hard_violation(&diagnostics) {
        return Err(execution_failure(language, inputs, diagnostics));
    }
    let Some(lifecycle_owner) = loaded.ast_stream.take().map(Arc::new) else {
        diagnostics.push(fatal_diagnostic(
            &request.data.uri,
            "cem.query.input_model_unsupported",
            "data input did not produce a lifecycle-owned native AST view",
        ));
        return Err(execution_failure(language, inputs, diagnostics));
    };

    let Some(runtime) = request.context.query_runtime_registry.adapter(language) else {
        diagnostics.push(fatal_diagnostic(
            &request.query.uri,
            "cem.query.runtime_unavailable",
            format!("no native query runtime is registered for `{language}`"),
        ));
        return Err(execution_failure(language, inputs, diagnostics));
    };
    let prepared = match runtime.prepare(QueryPreparationRequest {
        input_uri: &request.data.uri,
        input_identity,
        lifecycle_owner,
        query: &request.query,
        resolver_policy_stamp: &resolver_policy_stamp,
    }) {
        Ok(prepared) => prepared,
        Err(mut preparation_diagnostics) => {
            diagnostics.append(&mut preparation_diagnostics);
            return Err(execution_failure(language, inputs, diagnostics));
        }
    };
    diagnostics.extend(prepared.diagnostics);
    if request.context.abort_signal().is_aborted() {
        diagnostics.push(query_cancelled_diagnostic(
            &request.query.uri,
            request.context.abort_signal(),
        ));
        return Err(execution_failure(language, inputs, diagnostics));
    }
    if has_hard_violation(&diagnostics) {
        return Err(execution_failure(language, inputs, diagnostics));
    }

    let bindings: QueryNativeBindings<'_> = request
        .bindings
        .iter()
        .map(|(name, value)| (name.clone(), value.as_ref()))
        .collect();
    let result = match runtime.evaluate(QueryExecutionRequest {
        language,
        query_ast_owner: prepared.query_ast_owner.as_ref(),
        input_ast_owner: prepared.input_ast_owner.as_ref(),
        context_item: request.context_item.as_deref(),
        bindings: &bindings,
        namespace_bindings: &request.query.identity.namespaces,
        resolver_registry: &request.context.resolver_registry,
        resolver_policy: &request.context.resolver_policy,
        resolver_policy_stamp: &resolver_policy_stamp,
        safety_policy_stamp: &safety_policy_stamp,
        scope_policy: &scope_policy,
        abort_signal: request.context.abort_signal(),
        limits,
    }) {
        Ok(result) => result,
        Err(mut evaluation_diagnostics) => {
            diagnostics.append(&mut evaluation_diagnostics);
            return Err(execution_failure(language, inputs, diagnostics));
        }
    };

    if request.context.abort_signal().is_aborted() {
        diagnostics.push(query_cancelled_diagnostic(
            &request.query.uri,
            request.context.abort_signal(),
        ));
        return Err(execution_failure(language, inputs, diagnostics));
    }

    Ok(QueryRunResponse {
        language,
        inputs,
        result,
        diagnostics,
    })
}

fn query_budget_value(
    scope: &ScopeConfig,
    names: &[&str],
) -> Result<Option<u64>, QueryRunContractError> {
    let mut selected = None;
    for (name, value) in &scope.budgets {
        let normalized = name
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>();
        if !names.contains(&normalized.as_str()) {
            continue;
        }
        selected = Some(
            value
                .parse::<u64>()
                .map_err(|_| QueryRunContractError::BudgetInvalid { name: name.clone() })?,
        );
    }
    Ok(selected)
}

fn query_scope_policy(context: &EngineContext) -> ScopePolicy {
    let mut policy = ScopePolicy::host_root();
    if let Some(max_parallel_documents) = context.scheduler.max_parallel_documents {
        policy.cpu_workers = policy.cpu_workers.min(max_parallel_documents.max(1));
    }
    policy
}

fn query_safety_policy_stamp(limits: QueryExecutionLimits) -> String {
    format!(
        "cem-ml-query/1;result-items={};work-units={}",
        limits
            .max_result_items
            .map_or_else(|| "unbounded".to_owned(), |value| value.to_string()),
        limits
            .max_work_units
            .map_or_else(|| "unbounded".to_owned(), |value| value.to_string())
    )
}

fn has_hard_violation(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
}

fn fatal_diagnostic(uri: &str, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        code: code.to_owned(),
        severity: Severity::Fatal,
        message: message.into(),
        ..Diagnostic::default()
    }
}

fn query_cancelled_diagnostic(
    uri: &str,
    abort_signal: &crate::scheduler::AbortSignal,
) -> Diagnostic {
    Diagnostic {
        source_map: abort_signal.source_map(),
        ..fatal_diagnostic(
            uri,
            "cem.query.cancelled",
            "query execution was aborted by the host",
        )
    }
}

fn execution_failure(
    language: QueryLanguage,
    inputs: [String; 2],
    diagnostics: Vec<Diagnostic>,
) -> QueryRunError {
    QueryRunError::Execution(QueryRunFailure {
        language,
        inputs,
        diagnostics,
    })
}
