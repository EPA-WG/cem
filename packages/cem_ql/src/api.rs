//! Public CEM-QL entry points.

use std::collections::BTreeMap;

use cem_ml::content_cache::{CacheMode, ContentHash};
use cem_ml::diagnostics::Diagnostic;
use cem_ml::module_resolution::CemModuleUrlResolutionCapability;
use cem_ml::scheduler::{AbortSignal, ScopePolicy};
use cem_ml::schema::SchemaFrame;
use cem_ml::source::ByteRange;
use cem_ml::source_map::SourceMapStack;
use serde_json::json;

use crate::artifact::CompiledArtifact;
use crate::diagnostics::{self as ql_diagnostics, DATA_BINDING_MISSING, PARSE_ERROR};
use crate::eval::{Evaluator, Item, ItemStream, QueryContextScope};
use crate::ir::lower::IrLowerer;
use crate::ir::CompiledQuery;
use crate::parser::{Expression, Parser, SurfaceModule, SurfaceNode};
use crate::resolve::overlay::OverlayMap;
use crate::resolve::{Arity, FunctionKey, ImportPolicy, NameResolver, QNameKey};
use crate::semantic::validate_module_shape;
use crate::types::{FunctionSignature, TyConfig, Type, TypeChecker};

#[cfg(any(target_arch = "wasm32", test))]
mod json_boundary;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub const CEM_QL_EXPRESSION_CONTENT_TYPE: &str = "application/vnd.cem.query-expression+cem-ql";
pub const CEM_QL_EXPRESSION_SCHEMA_URI: &str = "https://cem.dev/ns/query/cem-ql/1#expression";
pub const PRIMARY_INPUT_BINDING: &str = "input";
const DEFAULT_EXPRESSION_QUEUE_SIZE: u32 = 256;

/// Compile a CEM-QL query module source string into a typed IR.
pub fn compile(source: &str, context: &CompileContext) -> Result<CompiledQuery, CompileError> {
    let parsed = parse(source);
    if let Some(diagnostic) = parsed
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CompileError::diagnostic(diagnostic));
    }
    let import_report = resolve_imports(&parsed.module, &context.import_policy);
    if let Some(diagnostic) = import_report
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CompileError::diagnostic(diagnostic));
    }
    let type_report = type_check(&parsed.module, context);
    if let Some(diagnostic) = type_report
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CompileError::diagnostic(diagnostic));
    }
    let lowered = IrLowerer::new()
        .with_policy_bindings(context.policy_bindings.keys().cloned())
        .lower_module(&parsed.module);
    if let Some(diagnostic) = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(CompileError::diagnostic(diagnostic));
    }
    Ok(lowered.query)
}

/// Evaluate a compiled query against a query context scope.
pub fn evaluate(query: &CompiledQuery, ctx: &EvaluationContext) -> ItemStream {
    Evaluator::evaluate(query, ctx)
}

/// Evaluate with a host-owned cooperative cancellation signal.
///
/// The signal remains outside the serializable query and evaluation records;
/// callers that own a longer-running operation can clone-share the same signal
/// used by resolver, scheduler, and output phases.
pub fn evaluate_with_abort(
    query: &CompiledQuery,
    ctx: &EvaluationContext,
    abort_signal: &AbortSignal,
) -> ItemStream {
    Evaluator::evaluate_with_abort(query, ctx, abort_signal)
}

/// Evaluate with the common operation-control core at one execution scope.
pub fn evaluate_with_control(
    query: &CompiledQuery,
    ctx: &EvaluationContext,
    control: &cem_ml::operation_control::OperationControl,
    scope: cem_ml::operation_control::ExecutionScopeId,
) -> ItemStream {
    Evaluator::evaluate_with_control(query, ctx, control, scope)
}

pub fn compile_expression(
    source: &str,
    context: &StandaloneExpressionContext,
) -> Result<CompiledExpression, ExpressionError> {
    let parsed = Parser::new(source).parse_module();
    let mut diagnostics = context.diagnostics.clone();
    diagnostics.extend(
        parsed
            .diagnostics
            .clone()
            .into_iter()
            .map(standalone_expression_source_diagnostic),
    );
    if has_hard_diagnostics(&diagnostics) {
        return Err(ExpressionError::diagnostics("parse", diagnostics));
    }

    let expression = match standalone_expression(&parsed.module, &mut diagnostics) {
        Some(expression) => expression,
        None => return Err(ExpressionError::diagnostics("parse", diagnostics)),
    };
    if has_hard_diagnostics(&diagnostics) {
        return Err(ExpressionError::diagnostics("parse", diagnostics));
    }

    let module = SurfaceModule {
        source: source.to_owned(),
        nodes: vec![SurfaceNode::Expression(expression.clone())],
    };
    let root_type = type_check_standalone_expression(&expression, &module, context);
    diagnostics.extend(
        root_type
            .diagnostics
            .into_iter()
            .map(|diagnostic| standalone_expression_type_diagnostic(diagnostic, context)),
    );
    if has_hard_diagnostics(&diagnostics) {
        return Err(ExpressionError::diagnostics("type-check", diagnostics));
    }

    let lowered = IrLowerer::new()
        .with_policy_bindings(context.bindings.keys().cloned())
        .with_policy_functions(context.functions.iter().map(function_key))
        .lower_module(&module);
    diagnostics.extend(
        lowered
            .diagnostics
            .clone()
            .into_iter()
            .map(|diagnostic| standalone_expression_type_diagnostic(diagnostic, context)),
    );
    if has_hard_diagnostics(&diagnostics) {
        return Err(ExpressionError::diagnostics("lower", diagnostics));
    }

    Ok(CompiledExpression {
        query: lowered.query,
        root_type: root_type.root_type,
        diagnostics,
        source_uri: context.source_uri.clone(),
        resolver_policy_stamp: context.resolver_policy_stamp.clone(),
        host_capability_profile: context.host_capability_profile.clone(),
    })
}

pub fn evaluate_expression(
    source: &str,
    context: &StandaloneExpressionContext,
) -> Result<ExpressionEvaluation, ExpressionError> {
    let compiled = compile_expression(source, context)?;
    let result = evaluate(
        &compiled.query,
        &EvaluationContext {
            scope: context.scope,
            scope_policy: context.scope_policy,
            diagnostics: context.diagnostics.clone(),
            policy_bindings: context.policy_bindings(),
            current_item: context.context_item.clone(),
            module_resolution: context.module_resolution.clone(),
        },
    );
    Ok(ExpressionEvaluation { compiled, result })
}

pub fn compile_artifact(
    source: &str,
    context: &CompileContext,
) -> Result<CompiledArtifact, CompileError> {
    compile(source, context).map(|query| CompiledArtifact::from_query_with_context(&query, context))
}

pub fn reload_artifact(artifact: &CompiledArtifact) -> Result<CompiledQuery, LoadError> {
    artifact.reload().map_err(LoadError::artifact)
}

pub fn reload_artifact_with_context(
    artifact: &CompiledArtifact,
    context: &CompileContext,
) -> Result<CompiledQuery, LoadError> {
    artifact
        .reload_with_context(context)
        .map_err(LoadError::artifact)
}

/// Parse-only entry point for tooling.
pub fn parse(source: &str) -> ParseResult {
    let mut parsed = Parser::new(source).parse_module();
    parsed
        .diagnostics
        .extend(validate_module_shape(&parsed.module));
    parsed
}

/// Resolve module import declarations without running full name binding.
pub fn resolve_imports(module: &SurfaceModule, import_policy: &ImportPolicy) -> Vec<Diagnostic> {
    NameResolver::new()
        .resolve_module_imports(module, import_policy)
        .diagnostics
}

/// Run strict or profile-configured static type checks for a parsed module.
pub fn type_check(module: &SurfaceModule, context: &CompileContext) -> Vec<Diagnostic> {
    let mut checker = TypeChecker::with_config(context.type_config.clone());
    checker.seed_runtime_import_surface(module);
    for name in context.policy_bindings.keys() {
        checker.declare_variable(crate::resolve::QNameKey::new(None, name.clone()), Type::Any);
    }
    checker.check_surface_module(module).diagnostics
}

/// Load a compiled binary artifact by content hash.
pub fn load(_hash: ContentHash, _ctx: &LoadContext) -> Result<CompiledQuery, LoadError> {
    Err(LoadError::unsupported(
        "CEM-QL artifact loading is not implemented yet",
    ))
}

#[derive(Debug, Clone)]
pub struct CompileContext {
    pub schema_frame: Option<SchemaFrame>,
    pub overlay: OverlayMap,
    pub import_policy: ImportPolicy,
    pub type_config: TyConfig,
    pub cache_mode: CacheMode,
    pub source_uri: Option<String>,
    pub expected_source_hash: Option<ContentHash>,
    pub diagnostics: Vec<Diagnostic>,
    pub source_map_base: SourceMapStack,
    pub policy_bindings: BTreeMap<String, ItemStream>,
}

impl Default for CompileContext {
    fn default() -> Self {
        Self {
            schema_frame: None,
            overlay: OverlayMap::default(),
            import_policy: ImportPolicy::default(),
            type_config: TyConfig::default(),
            cache_mode: CacheMode::Prod,
            source_uri: None,
            expected_source_hash: None,
            diagnostics: Vec::new(),
            source_map_base: SourceMapStack::default(),
            policy_bindings: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub code: &'static str,
    pub message: String,
}

impl CompileError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.unsupported",
            message: message.into(),
        }
    }

    pub fn diagnostic(diagnostic: &Diagnostic) -> Self {
        Self {
            code: "cem.ql.compile_failed",
            message: format!("{}: {}", diagnostic.code, diagnostic.message),
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, Clone)]
pub struct EvaluationContext {
    pub scope: QueryContextScope,
    pub scope_policy: ScopePolicy,
    pub diagnostics: Vec<Diagnostic>,
    pub policy_bindings: BTreeMap<String, ItemStream>,
    pub current_item: Option<Item>,
    /// Host-owned, scope-aware URL resolution capability.
    pub module_resolution: Option<CemModuleUrlResolutionCapability>,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root(),
            diagnostics: Vec::new(),
            policy_bindings: BTreeMap::new(),
            current_item: None,
            module_resolution: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StandaloneExpressionBinding {
    pub value: ItemStream,
    pub ty: Type,
}

impl StandaloneExpressionBinding {
    pub fn new(value: ItemStream, ty: Type) -> Self {
        Self { value, ty }
    }

    pub fn any(value: ItemStream) -> Self {
        Self {
            value,
            ty: Type::Any,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StandaloneExpressionContext {
    pub bindings: BTreeMap<String, StandaloneExpressionBinding>,
    pub functions: Vec<FunctionSignature>,
    pub context_item: Option<Item>,
    pub expected_type: Option<Type>,
    pub type_config: TyConfig,
    pub scope: QueryContextScope,
    pub scope_policy: ScopePolicy,
    pub diagnostics: Vec<Diagnostic>,
    pub source_uri: Option<String>,
    pub resolver_policy_stamp: Option<String>,
    pub host_capability_profile: Option<String>,
    pub module_resolution: Option<CemModuleUrlResolutionCapability>,
}

impl Default for StandaloneExpressionContext {
    fn default() -> Self {
        Self {
            bindings: BTreeMap::new(),
            functions: Vec::new(),
            context_item: None,
            expected_type: None,
            type_config: TyConfig::default(),
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(DEFAULT_EXPRESSION_QUEUE_SIZE),
            diagnostics: Vec::new(),
            source_uri: None,
            resolver_policy_stamp: None,
            host_capability_profile: None,
            module_resolution: None,
        }
    }
}

impl StandaloneExpressionContext {
    pub fn with_binding(
        mut self,
        name: impl Into<String>,
        binding: StandaloneExpressionBinding,
    ) -> Self {
        self.bindings.insert(name.into(), binding);
        self
    }

    pub fn with_input(mut self, value: ItemStream, ty: Type) -> Self {
        self.bindings.insert(
            PRIMARY_INPUT_BINDING.to_owned(),
            StandaloneExpressionBinding::new(value, ty),
        );
        self
    }

    pub fn with_context_item(mut self, item: Item) -> Self {
        self.context_item = Some(item);
        self
    }

    pub fn with_function(mut self, signature: FunctionSignature) -> Self {
        self.functions.push(signature);
        self
    }

    pub fn policy_bindings(&self) -> BTreeMap<String, ItemStream> {
        self.bindings
            .iter()
            .map(|(name, binding)| (name.clone(), binding.value.clone()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct CompiledExpression {
    pub query: CompiledQuery,
    pub root_type: Option<Type>,
    pub diagnostics: Vec<Diagnostic>,
    pub source_uri: Option<String>,
    pub resolver_policy_stamp: Option<String>,
    pub host_capability_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExpressionEvaluation {
    pub compiled: CompiledExpression,
    pub result: ItemStream,
}

#[derive(Debug, Clone)]
pub struct ExpressionError {
    pub code: &'static str,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl ExpressionError {
    pub fn diagnostics(stage: &'static str, diagnostics: Vec<Diagnostic>) -> Self {
        let message = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity.is_hard_violation())
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .unwrap_or_else(|| "standalone expression failed without a hard diagnostic".to_owned());
        Self {
            code: "cem.ql.expression_failed",
            message: format!("{stage}: {message}"),
            diagnostics,
        }
    }
}

impl std::fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ExpressionError {}

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub module: SurfaceModule,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default)]
pub struct LoadContext {
    pub expected_hash: Option<ContentHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub code: &'static str,
    pub message: String,
}

impl LoadError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "cem.ql.unsupported",
            message: message.into(),
        }
    }

    pub fn artifact(error: crate::artifact::ArtifactLoadError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for LoadError {}

fn type_check_standalone_expression(
    expression: &Expression,
    module: &SurfaceModule,
    context: &StandaloneExpressionContext,
) -> StandaloneExpressionTypeReport {
    let mut checker = TypeChecker::with_config(context.type_config.clone());
    checker.seed_runtime_import_surface(module);
    for (name, binding) in &context.bindings {
        checker.declare_variable(QNameKey::new(None, name.clone()), binding.ty.clone());
    }
    for signature in &context.functions {
        checker.register_function(signature.clone());
    }
    let root_type = checker.infer(expression);
    if let Some(expected_type) = &context.expected_type {
        checker.check_type(&root_type, expected_type, expression.range());
    }
    StandaloneExpressionTypeReport {
        root_type: Some(root_type),
        diagnostics: checker.diagnostics().to_vec(),
    }
}

fn function_key(signature: &FunctionSignature) -> FunctionKey {
    FunctionKey {
        name: signature.name.clone(),
        arity: Arity(signature.params.len().try_into().unwrap_or(u32::MAX)),
    }
}

#[derive(Debug, Clone)]
struct StandaloneExpressionTypeReport {
    root_type: Option<Type>,
    diagnostics: Vec<Diagnostic>,
}

fn standalone_expression(
    module: &SurfaceModule,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
    let mut expression = None;
    for node in &module.nodes {
        match node {
            SurfaceNode::Expression(candidate) => {
                if expression.is_some() {
                    diagnostics.push(standalone_expression_parse_diagnostic(
                        "standalone CEM-QL expression source must contain exactly one root expression",
                        candidate.range(),
                    ));
                } else {
                    expression = Some(candidate.clone());
                }
            }
            SurfaceNode::Module(_)
            | SurfaceNode::Import(_)
            | SurfaceNode::DeclareVariable(_)
            | SurfaceNode::DeclareFunction(_) => {
                diagnostics.push(standalone_expression_parse_diagnostic(
                    "standalone CEM-QL expression source must not contain module, import, or declare statements",
                    surface_node_range(node),
                ));
            }
        }
    }
    if expression.is_none() {
        diagnostics.push(standalone_expression_parse_diagnostic(
            "standalone CEM-QL expression source must contain one expression",
            ByteRange::new(0, 0),
        ));
    }
    expression
}

fn surface_node_range(node: &SurfaceNode) -> ByteRange {
    match node {
        SurfaceNode::Module(node) => node.range,
        SurfaceNode::Import(node) => node.range,
        SurfaceNode::DeclareVariable(node) => node.range,
        SurfaceNode::DeclareFunction(node) => node.range,
        SurfaceNode::Expression(node) => node.range(),
    }
}

fn standalone_expression_parse_diagnostic(
    message: impl Into<String>,
    range: ByteRange,
) -> Diagnostic {
    let mut diagnostic = ql_diagnostics::spanned_default(PARSE_ERROR, message, range);
    diagnostic.details = Some(json!({
        "behavior": "cem-ql-expression-report-fact",
        "factKind": "parse-error",
        "contract": "standalone-expression-parser",
        "recoverable": true,
        "fatal": false,
        "contentType": CEM_QL_EXPRESSION_CONTENT_TYPE,
        "schema": CEM_QL_EXPRESSION_SCHEMA_URI,
    }));
    diagnostic
}

fn standalone_expression_source_diagnostic(mut diagnostic: Diagnostic) -> Diagnostic {
    if diagnostic.code == PARSE_ERROR.as_str() && diagnostic.details.is_none() {
        diagnostic.details = Some(json!({
            "behavior": "cem-ql-expression-report-fact",
            "factKind": "parse-error",
            "contract": "standalone-expression-parser",
            "recoverable": true,
            "fatal": false,
            "contentType": CEM_QL_EXPRESSION_CONTENT_TYPE,
            "schema": CEM_QL_EXPRESSION_SCHEMA_URI,
        }));
    }
    diagnostic
}

fn standalone_expression_type_diagnostic(
    mut diagnostic: Diagnostic,
    context: &StandaloneExpressionContext,
) -> Diagnostic {
    if diagnostic.code == ql_diagnostics::UNKNOWN_VARIABLE.as_str() {
        diagnostic.code = DATA_BINDING_MISSING.into();
        diagnostic.details = Some(json!({
            "behavior": "cem-ql-expression-report-fact",
            "factKind": "data-binding-missing",
            "contract": "standalone-expression-binding",
            "recoverable": true,
            "fatal": false,
            "contentType": CEM_QL_EXPRESSION_CONTENT_TYPE,
            "schema": CEM_QL_EXPRESSION_SCHEMA_URI,
            "availableBindings": context.bindings.keys().cloned().collect::<Vec<_>>(),
        }));
    }
    diagnostic
}

fn has_hard_diagnostics(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::eval::{AtomValue, Item};
    use crate::resolve::QNameKey;
    use crate::types::{AtomType, FunctionSignature};

    use super::*;

    #[test]
    fn standalone_expression_evaluates_against_typed_input_binding() {
        let context = StandaloneExpressionContext::default().with_input(
            ItemStream::from_items(vec![
                row("Ada", "required"),
                row("Lin", "recommended"),
                row("Max", "deprecated"),
            ]),
            Type::Any,
        );

        let evaluation = evaluate_expression("input.name", &context)
            .expect("standalone expression should evaluate");

        assert_eq!(
            evaluation.result.items,
            vec![
                Item::Atomic(AtomValue::String("Ada".to_owned())),
                Item::Atomic(AtomValue::String("Lin".to_owned())),
                Item::Atomic(AtomValue::String("Max".to_owned())),
            ]
        );
        assert_eq!(evaluation.compiled.query.source, "input.name");
    }

    #[test]
    fn standalone_expression_uses_top_level_current_item() {
        let context = StandaloneExpressionContext::default()
            .with_context_item(Item::Atomic(AtomValue::Integer(7)));

        let evaluation = evaluate_expression(".", &context)
            .expect("standalone expression should evaluate current item");

        assert_eq!(
            evaluation.result.items,
            vec![Item::Atomic(AtomValue::Integer(7))]
        );
    }

    #[test]
    fn standalone_expression_rejects_module_wrapper() {
        let error = compile_expression(
            r#"module "https://example.test/query"
1"#,
            &StandaloneExpressionContext::default(),
        )
        .expect_err("standalone expression must reject module syntax");

        assert!(error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.parse_error"
                && diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("contract"))
                    .and_then(serde_json::Value::as_str)
                    == Some("standalone-expression-parser")));
    }

    #[test]
    fn standalone_expression_checks_expected_type() {
        let mut context = StandaloneExpressionContext::default();
        context.expected_type = Some(Type::atom(AtomType::Boolean));

        let error = compile_expression("1", &context)
            .expect_err("integer expression should not satisfy expected boolean");

        assert!(error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.type_error"));
    }

    #[test]
    fn standalone_expression_reports_missing_data_binding() {
        let context =
            StandaloneExpressionContext::default().with_input(ItemStream::empty(), Type::Any);

        let error = compile_expression("missingBinding", &context)
            .expect_err("unbound standalone expression name should fail");

        let diagnostic = error
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "cem.ql.data_binding_missing")
            .unwrap_or_else(|| panic!("missing data binding diagnostic: {error:#?}"));
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("contract")),
            Some(&json!("standalone-expression-binding"))
        );
        assert_eq!(
            diagnostic
                .details
                .as_ref()
                .and_then(|details| details.get("availableBindings")),
            Some(&json!(["input"]))
        );
    }

    #[test]
    fn standalone_expression_accepts_host_function_signatures() {
        let context = StandaloneExpressionContext::default()
            .with_input(ItemStream::empty(), Type::Any)
            .with_function(FunctionSignature {
                name: QNameKey::new(None, "format"),
                params: vec![Type::Any],
                ret: Type::Any,
            });

        let compiled = compile_expression("format(input)", &context)
            .expect("standalone expression should accept host helper signatures");

        assert_eq!(compiled.query.source, "format(input)");
        assert!(compiled
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.severity.is_hard_violation()));
    }

    fn row(name: &str, tier: &str) -> Item {
        Item::Record(BTreeMap::from([
            (
                "name".to_owned(),
                vec![Item::Atomic(AtomValue::String(name.to_owned()))],
            ),
            (
                "tier".to_owned(),
                vec![Item::Atomic(AtomValue::String(tier.to_owned()))],
            ),
        ]))
    }
}
