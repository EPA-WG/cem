//! Opt-in named CEMT XPath bodies callable through CEM-QL's native registry.
//! Scalar parameters are explicitly declared; `any` accepts retained native
//! XPath sequences only. Existing host adapters remain native-XDM-only.
use crate::{
    eval::{AtomValue, BudgetAxis, EvalError, ItemStream},
    native::{NativeFunctionRegistry, NativeQueryFunction, NativeQueryRequest},
};
use cem_ml::{
    content_cache::ContentHash,
    diagnostics::{Diagnostic, Severity},
    engine::{FormatIdentity, TemplateInput},
    resolver::{ResolverPolicy, ResolverRegistry},
    schema::registry::{CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI},
    transform_template::{
        invoke_transform_template_xpath, parse_cem_native_template_module_options,
        TransformTemplateModuleParamType as ParamType, TransformTemplateModuleParseRequest,
        TransformTemplateModuleVisibility, TransformTemplateRuntimeContext,
        TransformTemplateXPathHostBindings, TransformTemplateXPathInvocation,
    },
    validation::xpath::{
        artifact::{XPathArtifactLoadContext, XPathCompiledArtifact},
        XPathAtomicValue, XPathEvaluationLimits, XPathExpressionAst, XPathInvocationHost,
        XPathResultItem, XPathResultSequence,
    },
};
use std::{collections::BTreeSet, sync::Arc};

mod value;
pub use value::XPathQueryItem;
pub mod companion;

const MAX_SOURCE_BYTES: usize = 32 * 1024;
const MAX_FUNCTIONS: usize = 64;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Parameter {
    pub(crate) name: String,
    pub(crate) kind: ParamType,
    pub(crate) nullable: bool,
}

/// Immutable local function library. It does not install capabilities by itself.
#[derive(Debug, Clone)]
pub struct CemtXPathFunctions {
    functions: Vec<CemtXPathFunction>,
    source_hash: ContentHash,
    source_uri: String,
}

#[derive(Debug, Clone)]
pub struct CemtXPathFunction {
    name: String,
    parameters: Vec<Parameter>,
    returns: ParamType,
    nullable: bool,
    result_contract: ResultContract,
    invocation: TransformTemplateXPathInvocation,
    artifact: XPathCompiledArtifact,
}

impl CemtXPathFunction {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn arity(&self) -> usize {
        self.parameters.len()
    }
    pub fn expression(&self) -> &XPathExpressionAst {
        &self.invocation.expression
    }
    pub fn artifact(&self) -> &XPathCompiledArtifact {
        &self.artifact
    }
}

impl CemtXPathFunctions {
    pub fn compile(source: &str, source_uri: &str) -> Result<Self, Vec<Diagnostic>> {
        let invalid = |message: String| {
            vec![Diagnostic {
                uri: Some(source_uri.into()),
                code: "cem.ql.xpath_function_declaration".into(),
                severity: Severity::Error,
                message,
                ..Default::default()
            }]
        };
        if source.len() > MAX_SOURCE_BYTES || source_uri.len() > MAX_SOURCE_BYTES {
            return Err(invalid(
                "XPath function library exceeds source limit".into(),
            ));
        }
        let parsed =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: source_uri.into(),
                    bytes: source.as_bytes().to_vec(),
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.into()),
                        schema: Some(CEM_TRANSFORM_SCHEMA_URI.into()),
                        ..Default::default()
                    }),
                    root_scope: Default::default(),
                },
            });
        if parsed
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(parsed.diagnostics);
        }
        let options = parsed.module_options;
        if !options.imports.is_empty() {
            return Err(invalid(
                "XPath function-library imports require an explicit resolved companion closure"
                    .into(),
            ));
        }
        if options.xpath_invocations.len() > MAX_FUNCTIONS {
            return Err(invalid("XPath function count exceeds limit".into()));
        }
        let source_hash = ContentHash::from_blake3(source.as_bytes());
        let mut functions = Vec::new();
        let mut names = BTreeSet::new();
        for mut invocation in options.xpath_invocations {
            let declarations: Vec<_> = options
                .functions
                .iter()
                .filter(|decl| decl.name == invocation.owner_function)
                .collect();
            let [declaration] = declarations.as_slice() else {
                return Err(invalid(format!(
                    "XPath body `{}` requires one named function declaration",
                    invocation.owner_function
                )));
            };
            if declaration.visibility != TransformTemplateModuleVisibility::Public {
                continue;
            }
            if !names.insert(declaration.name.clone()) || declaration.params.len() >= 255 {
                return Err(invalid(
                    "duplicate XPath function or unsupported arity".into(),
                ));
            }
            if !supported_type(declaration.return_type) {
                return Err(invalid(
                    "XPath function return type must be scalar or native any".into(),
                ));
            }
            let mut parameters = Vec::new();
            let mut parameter_names = BTreeSet::new();
            for param in &declaration.params {
                if !supported_type(param.value_type)
                    || param.default_value.is_some()
                    || !param.required
                    || !parameter_names.insert(param.name.clone())
                {
                    return Err(invalid("XPath parameters must be unique required scalar/native arguments without defaults".into()));
                }
                parameters.push(Parameter {
                    name: param.name.clone(),
                    kind: param.value_type,
                    nullable: param.nullable,
                });
            }
            for binding in invocation
                .variable_bindings
                .iter()
                .map(|b| &b.host_binding)
                .chain(invocation.context_binding.iter())
            {
                if !parameter_names.contains(binding) {
                    return Err(invalid(format!(
                        "XPath host binding `{binding}` has no declared parameter"
                    )));
                }
            }
            let result_contract = ResultContract::parse(&invocation.expected_result.sequence_type)
                .ok_or_else(|| invalid("unsupported XPath function result sequence type".into()))?;
            if result_contract.item_type.starts_with("xs:")
                && invocation
                    .static_context
                    .namespaces
                    .get("xs")
                    .is_some_and(|uri| uri != "http://www.w3.org/2001/XMLSchema")
            {
                return Err(invalid(
                    "XPath result xs prefix must identify XML Schema types".into(),
                ));
            }
            let artifact =
                XPathCompiledArtifact::compile(&invocation.expression, source_hash.clone())
                    .map_err(|e| invalid(e.to_string()))?;
            invocation.expression = Arc::new(
                artifact
                    .reload(&XPathArtifactLoadContext {
                        expected_source_hash: source_hash.clone(),
                        invocation_host: XPathInvocationHost::Cemt,
                    })
                    .map_err(|e| invalid(e.to_string()))?,
            );
            functions.push(CemtXPathFunction {
                name: declaration.name.clone(),
                parameters,
                returns: declaration.return_type,
                nullable: declaration.nullable,
                result_contract,
                invocation,
                artifact,
            });
        }
        Ok(Self {
            functions,
            source_hash,
            source_uri: source_uri.into(),
        })
    }

    pub fn source_hash(&self) -> &ContentHash {
        &self.source_hash
    }
    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }

    pub fn len(&self) -> usize {
        self.functions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
    pub fn functions(&self) -> &[CemtXPathFunction] {
        &self.functions
    }

    /// Install atomically into an explicit registry; no global/default registry
    /// or portable CEMT artifact changes. The supplied resolver policy stays owned.
    pub fn install(
        &self,
        registry: &mut NativeFunctionRegistry,
        resolvers: Arc<ResolverRegistry>,
        policy: Arc<ResolverPolicy>,
    ) -> Result<(), String> {
        self.install_with_limits(
            registry,
            resolvers,
            policy,
            XPathEvaluationLimits::default(),
        )
    }

    /// Per-invocation limits belong to the installing host, never to library source.
    pub fn install_with_limits(
        &self,
        registry: &mut NativeFunctionRegistry,
        resolvers: Arc<ResolverRegistry>,
        policy: Arc<ResolverPolicy>,
        limits: XPathEvaluationLimits,
    ) -> Result<(), String> {
        let mut candidate = registry.clone();
        for function in &self.functions {
            candidate.register(
                function.name.clone(),
                function.arity(),
                InstalledFunction {
                    function: function.clone(),
                    resolvers: resolvers.clone(),
                    policy: policy.clone(),
                    limits,
                },
            )?;
        }
        *registry = candidate;
        Ok(())
    }
}

pub(crate) fn supported_type(kind: ParamType) -> bool {
    matches!(
        kind,
        ParamType::Any
            | ParamType::String
            | ParamType::Boolean
            | ParamType::Integer
            | ParamType::Number
    )
}

struct InstalledFunction {
    limits: XPathEvaluationLimits,
    function: CemtXPathFunction,
    resolvers: Arc<ResolverRegistry>,
    policy: Arc<ResolverPolicy>,
}
impl std::fmt::Debug for InstalledFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstalledXPathFunction")
            .field("name", &self.function.name)
            .finish_non_exhaustive()
    }
}

impl NativeQueryFunction for InstalledFunction {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        let function = &self.function;
        if request.arguments.len() != function.arity() {
            return request.raise(
                "cem.ql.xpath_function_argument",
                "XPath function arity mismatch",
            );
        }
        let mut bindings = TransformTemplateXPathHostBindings::default();
        let mut item_count = 0u64;
        for (parameter, argument) in function.parameters.iter().zip(request.arguments) {
            item_count = item_count.saturating_add(argument.items.len() as u64);
            if item_count > request.max_result_items {
                return limit_failure(&request, "XPath argument item limit exceeded");
            }
            let values = match bind_argument(parameter, argument, &request) {
                Ok(values) => values,
                Err(message) => return request.raise("cem.ql.xpath_function_argument", message),
            };
            item_count = item_count.saturating_add(values.len().saturating_sub(argument.items.len()) as u64);
            if item_count > request.max_result_items {
                return limit_failure(&request, "Expanded XPath argument item limit exceeded");
            }
            if function.invocation.context_binding.as_deref() == Some(parameter.name.as_str()) {
                let [value] = values.as_slice() else {
                    return request.raise(
                        "cem.ql.xpath_function_argument",
                        "XPath context requires exactly one item",
                    );
                };
                bindings
                    .context_items
                    .insert(parameter.name.clone(), value.clone());
            }
            bindings.variable_sequences.insert(
                parameter.name.clone(),
                XPathResultSequence {
                    sequence_type: "item()*".into(),
                    items: values,
                },
            );
        }
        let result = invoke_transform_template_xpath(
            &function.invocation,
            &bindings,
            XPathEvaluationLimits {
                max_sequence_items: Some(
                    self.limits
                        .max_sequence_items
                        .map_or(request.max_result_items, |limit| {
                            limit.min(request.max_result_items)
                        }),
                ),
                ..self.limits
            },
            TransformTemplateRuntimeContext {
                resolver_registry: &self.resolvers,
                resolver_policy: &self.policy,
                operation_control: request.control,
                execution_scope: request.scope,
            },
        );
        match result {
            Err(diagnostics) => invocation_failure(&request, diagnostics),
            Ok(result) => {
                let items = result.sequence.items;
                if !function.result_contract.accepts(&items)
                    || !declared_result_accepts(function.returns, function.nullable, &items)
                {
                    let mut failed = request.raise(
                        "cem.ql.xpath_function_result_type",
                        "XPath function returned a value outside its declared type/cardinality",
                    );
                    failed.diagnostics[0].source_map = Some(function.invocation.source_map.clone());
                    failed.diagnostics[0].uri = Some(function.expression().source.uri.clone());
                    return failed;
                }
                ItemStream::from_items(items.into_iter().map(XPathQueryItem::wrap).collect())
            }
        }
    }
}

fn limit_failure(request: &NativeQueryRequest<'_>, message: &str) -> ItemStream {
    let mut failed = request.raise("cem.ql.xpath_function_limit", message);
    failed.error = Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage));
    failed
}

pub(crate) fn bind_argument(
    parameter: &Parameter,
    argument: &ItemStream,
    request: &NativeQueryRequest<'_>,
) -> Result<Vec<XPathResultItem>, String> {
    if parameter.kind != ParamType::Any
        && (argument.items.len() > 1 || (argument.items.is_empty() && !parameter.nullable))
    {
        return Err(format!(
            "parameter `{}` requires one declared scalar",
            parameter.name
        ));
    }
    argument
        .items
        .iter()
        .map(|item| {
            if let Some(native) = item.view().and_then(|v| v.downcast_ref::<XPathQueryItem>()) {
                if parameter.kind == ParamType::Any
                    || item_has_type(native.xpath_item(), parameter.kind)
                {
                    return Ok(vec![native.xpath_item().clone()]);
                }
            }
            if parameter.kind == ParamType::Any {
                if let Some(targets) = crate::eval::values::reference_values(item) {
                    return bind_argument(
                        parameter,
                        &ItemStream::from_items(targets.to_vec()),
                        request,
                    );
                }
                if let Some(node) = crate::eval::imported_xpath_node(item) {
                    return node.map(|node| vec![XPathResultItem::from_native_node(node)]);
                }
            }
            if let Some(values) =
                crate::eval::xpath_values::native_items(item, request.query_scope, &mut || {
                    request
                        .control
                        .check_scope(request.scope)
                        .map_err(|e| e.to_string())
                })
            {
                let values = values?;
                if parameter.kind == ParamType::Any
                    || values.len() == 1
                        && values
                            .iter()
                            .all(|item| item_has_type(item, parameter.kind))
                {
                    return Ok(values);
                }
                return Err(format!(
                    "parameter `{}` rejects the native value type/cardinality",
                    parameter.name
                ));
            }
            let atom = item.atom().filter(|_| {
                item.view()
                    .is_none_or(|v| v.kind() == crate::eval::QueryItemViewKind::Atomic)
            });
            let Some(atom) = atom.as_ref() else {
                return Err(format!(
                    "parameter `{}` requires an explicit scalar or retained XPath item",
                    parameter.name
                ));
            };
            let (type_name, lexical_value) = match (parameter.kind, atom) {
                (ParamType::String | ParamType::Any, AtomValue::String(text)) => {
                    ("xs:string", text.clone())
                }
                (ParamType::Boolean | ParamType::Any, AtomValue::Boolean(value)) => {
                    ("xs:boolean", value.to_string())
                }
                (
                    ParamType::Integer | ParamType::Number | ParamType::Any,
                    AtomValue::Integer(value),
                ) => ("xs:integer", value.to_string()),
                (ParamType::Number | ParamType::Any, AtomValue::Decimal(value))
                    if valid_decimal(value) =>
                {
                    ("xs:decimal", value.clone())
                }
                (ParamType::Number | ParamType::Any, AtomValue::Double(value)) => (
                    "xs:double",
                    if value.is_nan() {
                        "NaN".into()
                    } else if *value == f64::INFINITY {
                        "INF".into()
                    } else if *value == f64::NEG_INFINITY {
                        "-INF".into()
                    } else {
                        value.to_string()
                    },
                ),
                (ParamType::Any, AtomValue::AnyUri(value)) => ("xs:anyURI", value.clone()),
                (ParamType::Any, AtomValue::Null) => return Ok(Vec::new()),
                _ => {
                    return Err(format!(
                        "parameter `{}` does not accept this value as {}",
                        parameter.name,
                        parameter.kind.as_contract_name()
                    ))
                }
            };
            Ok(vec![XPathResultItem::Atomic {
                value: XPathAtomicValue {
                    type_name: type_name.into(),
                    lexical_value,
                    namespace_uri: None,
                    local_name: None,
                },
                source_map: request.source_map.clone(),
            }])
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.into_iter().flatten().collect())
}

fn valid_decimal(value: &str) -> bool {
    let value = value.strip_prefix(['+', '-']).unwrap_or(value);
    let mut dots = 0;
    let mut digits = 0;
    value.bytes().all(|byte| match byte {
        b'0'..=b'9' => {
            digits += 1;
            true
        }
        b'.' => {
            dots += 1;
            dots <= 1
        }
        _ => false,
    }) && digits > 0
}

pub(crate) fn item_has_type(item: &XPathResultItem, kind: ParamType) -> bool {
    if kind == ParamType::Any {
        return match item {
            XPathResultItem::Node { .. } => item.native_node().is_some(),
            XPathResultItem::Atomic { .. } => true,
            XPathResultItem::Map { entries, .. } => entries.iter().all(|entry| {
                entry
                    .value
                    .items
                    .iter()
                    .all(|item| item_has_type(item, kind))
            }),
            XPathResultItem::Array { members, .. } => members
                .iter()
                .all(|member| member.items.iter().all(|item| item_has_type(item, kind))),
            XPathResultItem::Function { .. } => false,
        };
    }
    let XPathResultItem::Atomic { value, .. } = item else {
        return false;
    };
    match kind {
        ParamType::String => value.type_name == "xs:string",
        ParamType::Boolean => value.type_name == "xs:boolean",
        ParamType::Integer => value.type_name == "xs:integer",
        ParamType::Number => matches!(
            value.type_name.as_str(),
            "xs:integer" | "xs:decimal" | "xs:float" | "xs:double"
        ),
        _ => false,
    }
}

fn declared_result_accepts(kind: ParamType, nullable: bool, items: &[XPathResultItem]) -> bool {
    if kind == ParamType::Any {
        return items.iter().all(|item| item_has_type(item, kind));
    }
    if items.is_empty() {
        return nullable;
    }
    items.len() == 1 && item_has_type(&items[0], kind)
}

/// Closed host result contract, not a second XPath sequence-type parser.
#[derive(Debug, Clone)]
struct ResultContract {
    item_type: String,
    min: usize,
    max: usize,
}
impl ResultContract {
    fn parse(source: &str) -> Option<Self> {
        let source = source.trim();
        if source == "empty-sequence()" {
            return Some(Self {
                item_type: "item()".into(),
                min: 0,
                max: 0,
            });
        }
        let (item_type, min, max) = match source.as_bytes().last()? {
            b'?' => (&source[..source.len() - 1], 0, 1),
            b'*' => (&source[..source.len() - 1], 0, usize::MAX),
            b'+' => (&source[..source.len() - 1], 1, usize::MAX),
            _ => (source, 1, 1),
        };
        if !matches!(
            item_type,
            "item()"
                | "node()"
                | "map(*)"
                | "array(*)"
                | "xs:string"
                | "xs:boolean"
                | "xs:integer"
                | "xs:decimal"
                | "xs:float"
                | "xs:double"
                | "xs:anyURI"
                | "xs:untypedAtomic"
        ) {
            return None;
        }
        Some(Self {
            item_type: item_type.into(),
            min,
            max,
        })
    }
    fn accepts(&self, items: &[XPathResultItem]) -> bool {
        items.len() >= self.min && items.len() <= self.max && items.iter().all(|item| {
            match self.item_type.as_str() {
                "item()" => item_has_type(item, ParamType::Any),
                "node()" => item.native_node().is_some(),
                "map(*)" => matches!(item, XPathResultItem::Map { .. }),
                "array(*)" => matches!(item, XPathResultItem::Array { .. }),
                name => matches!(item, XPathResultItem::Atomic { value, .. }
                    if value.type_name == name || (name == "xs:decimal" && value.type_name == "xs:integer")),
            }
        })
    }
}

pub(crate) fn invocation_failure(
    request: &NativeQueryRequest<'_>,
    diagnostics: Vec<Diagnostic>,
) -> ItemStream {
    let mut failed = request.raise("cem.ql.xpath_function_failed", "XPath function failed");
    // Resource/capability failures must not become catchable data errors.
    if diagnostics.iter().any(|d| {
        matches!(
            d.code.as_str(),
            "cem.xpath.sequence_item_limit_exceeded" | "cem.xpath.import_limit_exceeded"
        )
    }) {
        failed.error = Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage));
    } else if diagnostics
        .iter()
        .any(|d| d.code == "cem.xpath.function_depth_exceeded")
    {
        failed.error = Some(EvalError::BudgetExceeded(BudgetAxis::CallDepth));
    } else if diagnostics
        .iter()
        .any(|d| d.code == "cem.xpath.text_byte_limit_exceeded")
    {
        failed.error = Some(EvalError::BudgetExceeded(BudgetAxis::XPathTextBytes));
    } else if diagnostics.iter().any(|d| {
        matches!(
            d.code.as_str(),
            "cem.xpath.work_limit_exceeded" | "cem.xpath.regex_limit_exceeded"
        )
    }) {
        failed.error = Some(EvalError::BudgetExceeded(BudgetAxis::XPathWorkUnits));
    } else if diagnostics.iter().any(|d| {
        matches!(
            d.code.as_str(),
            "cem.xpath.evaluation_unsupported"
                | "cem.xpath.regex_unsupported"
                | "cem.xpath.control_failure"
                | "cem.xpath.node_order_cross_owner_unsupported"
                | "cem.xpath.module_url_unavailable"
                | "cem.xpath.module_url_referrer_unavailable"
        )
    }) {
        failed.error = Some(EvalError::Unsupported(
            "XPath capability or operation control unavailable",
        ));
    } else if let Some(first) = diagnostics.first() {
        failed.error = Some(EvalError::Raised {
            code: first.code.clone().into_boxed_str(),
            message: first.message.clone().into_boxed_str(),
        });
    }
    failed.diagnostics = diagnostics;
    failed
}
