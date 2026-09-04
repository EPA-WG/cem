//! Layer 6: pull-based evaluator.

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::module_resolution::{
    CemModuleUrlResolutionCapability, CemResolutionContextHandle,
};
use cem_ml::operation_control::{
    ControlError, ControlFailure, ControlTerminalClass, ExecutionScopeId, OperationControl,
    SafePointPoller, ROOT_EXECUTION_SCOPE_ID,
};
use cem_ml::scheduler::{AbortSignal, ScopePolicy};
use cem_ml::source_map::SourceMapStack;

use crate::api::EvaluationContext;
use crate::diagnostics::{
    DiagnosticCode, ABORTED, BUDGET_EXCEEDED, TYPE_ERROR, UNKNOWN_FUNCTION, UNKNOWN_VARIABLE,
};
use crate::ir::{CompiledQuery, IrId, IrNode, IrRecordKey};
use crate::parser::{BinaryOp, QuantifierKind, SetOp, UnaryOp};
use crate::resolve::BindingId;
use crate::types::Type;

pub mod pipeline;
pub mod set_ops;
pub mod types_runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryContextScope(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryItemViewKind {
    Atomic,
    Record,
    Array,
    Node,
}

pub trait QueryItemView: fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn representation_id(&self) -> &'static str;
    fn identity(&self) -> String;
    fn kind(&self) -> QueryItemViewKind;
    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        None
    }
    fn field(&self, _name: &str) -> Option<Vec<Item>> {
        None
    }
    fn members(&self) -> Option<Vec<Item>> {
        None
    }
    fn atom(&self) -> Option<AtomValue> {
        None
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        None
    }
}

#[derive(Clone)]
pub struct NativeItemView {
    view: Arc<dyn QueryItemView>,
}

impl NativeItemView {
    pub fn new(view: impl QueryItemView + 'static) -> Self {
        Self {
            view: Arc::new(view),
        }
    }

    pub fn representation_id(&self) -> &'static str {
        self.view.representation_id()
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.view.as_any().downcast_ref::<T>()
    }

    pub fn identity(&self) -> String {
        self.view.identity()
    }

    pub fn kind(&self) -> QueryItemViewKind {
        self.view.kind()
    }

    pub fn field(&self, name: &str) -> Option<Vec<Item>> {
        self.view.field(name)
    }

    pub fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        self.view.fields()
    }

    pub fn members(&self) -> Option<Vec<Item>> {
        self.view.members()
    }

    pub fn atom(&self) -> Option<AtomValue> {
        self.view.atom()
    }

    pub fn source_map(&self) -> Option<SourceMapStack> {
        self.view.source_map()
    }
}

impl fmt::Debug for NativeItemView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeItemView")
            .field("representation_id", &self.representation_id())
            .field("identity", &self.identity())
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

impl PartialEq for NativeItemView {
    fn eq(&self, other: &Self) -> bool {
        self.representation_id() == other.representation_id() && self.identity() == other.identity()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Node(String),
    Atomic(AtomValue),
    Record(BTreeMap<String, Vec<Item>>),
    Array(Vec<Item>),
    Native(NativeItemView),
    Lambda(IrId),
    Resource(ResourceHandle),
}

impl Item {
    pub fn native(view: impl QueryItemView + 'static) -> Self {
        Self::Native(NativeItemView::new(view))
    }

    pub fn view(&self) -> Option<&NativeItemView> {
        match self {
            Self::Native(view) => Some(view),
            _ => None,
        }
    }

    pub fn module_resolution_node_identity(&self) -> Option<String> {
        match self {
            Self::Node(identity) => Some(format!("node:{identity}")),
            Self::Native(view) if view.kind() == QueryItemViewKind::Node => Some(format!(
                "native:{}:{}",
                view.representation_id(),
                view.identity()
            )),
            _ => None,
        }
    }

    pub fn atom(&self) -> Option<AtomValue> {
        match self {
            Self::Atomic(atom) => Some(atom.clone()),
            Self::Native(view) => view.atom(),
            _ => None,
        }
    }

    pub fn identity(&self) -> Option<String> {
        match self {
            Self::Native(view) => Some(view.identity()),
            _ => None,
        }
    }

    pub fn source_map(&self) -> Option<SourceMapStack> {
        match self {
            Self::Native(view) => view.source_map(),
            _ => None,
        }
    }

    pub fn members(&self) -> Option<Vec<Item>> {
        match self {
            Self::Array(items) => Some(items.clone()),
            Self::Native(view) => view.members(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHandle {
    pub id: String,
    pub content_type: String,
    pub schema: Option<String>,
    pub roles: Vec<String>,
    pub fail_accessor: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AtomValue {
    String(String),
    Integer(i64),
    Decimal(String),
    Double(f64),
    Boolean(bool),
    AnyUri(String),
    Null,
}

#[derive(Debug, Clone)]
pub struct ItemStream {
    pub items: Vec<Item>,
    pub diagnostics: Vec<Diagnostic>,
    pub error: Option<EvalError>,
    cursor: usize,
}

impl Default for ItemStream {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for ItemStream {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items && self.error == other.error
    }
}

impl ItemStream {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            diagnostics: Vec::new(),
            error: None,
            cursor: 0,
        }
    }

    pub fn once(item: Item) -> Self {
        Self::from_items(vec![item])
    }

    pub fn from_items(items: Vec<Item>) -> Self {
        Self {
            items,
            diagnostics: Vec::new(),
            error: None,
            cursor: 0,
        }
    }

    pub fn failed(error: EvalError, diagnostic: Diagnostic) -> Self {
        Self {
            items: Vec::new(),
            diagnostics: vec![diagnostic],
            error: Some(error),
            cursor: 0,
        }
    }

    pub fn next_item(&mut self) -> Option<Result<Item, EvalError>> {
        if self.cursor < self.items.len() {
            let item = self.items[self.cursor].clone();
            self.cursor += 1;
            Some(Ok(item))
        } else {
            self.error.take().map(Err)
        }
    }

    pub fn extend_diagnostics(&mut self, other: ItemStream) {
        self.diagnostics.extend(other.diagnostics);
        if self.error.is_none() {
            self.error = other.error;
        }
    }

    pub fn append_stream(&mut self, mut other: ItemStream) {
        self.items.append(&mut other.items);
        self.extend_diagnostics(other);
    }

    fn with_context(mut self, ctx: &EvalCtx<'_>) -> Self {
        self.diagnostics.extend(ctx.diagnostics.clone());
        if self.error.is_none() {
            self.error = ctx.error.clone();
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BudgetAxis {
    ItemsPerStage,
    CallDepth,
    FunctionCalls,
    ClosureSize,
    RegexBacktrack,
    ExternalFetches,
}

impl BudgetAxis {
    pub fn as_str(self) -> &'static str {
        match self {
            BudgetAxis::ItemsPerStage => "items-per-stage",
            BudgetAxis::CallDepth => "call-depth",
            BudgetAxis::FunctionCalls => "function-calls",
            BudgetAxis::ClosureSize => "closure-size",
            BudgetAxis::RegexBacktrack => "regex-backtrack",
            BudgetAxis::ExternalFetches => "external-fetches",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    Cancelled,
    Control(ControlFailure),
    BudgetExceeded(BudgetAxis),
    Unsupported(&'static str),
    TypeError(&'static str),
}

#[derive(Debug, Clone, Default)]
pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(query: &CompiledQuery, context: &EvaluationContext) -> ItemStream {
        let abort_signal = AbortSignal::new();
        Self::evaluate_with_abort(query, context, &abort_signal)
    }

    pub fn evaluate_with_abort<'a>(
        query: &'a CompiledQuery,
        context: &EvaluationContext,
        abort_signal: &'a AbortSignal,
    ) -> ItemStream {
        let control = OperationControl::new(abort_signal.clone());
        Self::evaluate_with_control(query, context, &control, ROOT_EXECUTION_SCOPE_ID)
    }

    pub fn evaluate_with_control(
        query: &CompiledQuery,
        context: &EvaluationContext,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> ItemStream {
        let mut ctx = EvalCtx::new(query, context, control, scope);
        let mut stream = match ctx.force_safe_point(query.tree.root) {
            Ok(()) => ctx.eval_id(query.tree.root),
            Err(controlled) => controlled,
        };
        if let Err(controlled) = ctx.force_safe_point(query.tree.root) {
            // Evaluation values remain internal until the operation-control
            // acceptance boundary succeeds. A terminal control cause must
            // never expose the prefix accumulated before its safe point.
            stream = controlled;
        }
        stream.with_context(&ctx)
    }
}

pub(crate) struct EvalCtx<'a> {
    query: &'a CompiledQuery,
    safe_points: SafePointPoller,
    scopes: Vec<HashMap<BindingId, ItemStream>>,
    globals: HashMap<BindingId, IrId>,
    functions: HashMap<BindingId, IrId>,
    current_items: Vec<Item>,
    counters: HashMap<BudgetAxis, u64>,
    limits: HashMap<BudgetAxis, u64>,
    call_depth: u64,
    diagnostics: Vec<Diagnostic>,
    error: Option<EvalError>,
    module_resolution: Option<CemModuleUrlResolutionCapability>,
}

impl<'a> EvalCtx<'a> {
    fn new(
        query: &'a CompiledQuery,
        context: &EvaluationContext,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Self {
        let mut ctx = Self {
            query,
            safe_points: SafePointPoller::new(control.clone(), scope),
            scopes: vec![HashMap::new()],
            globals: HashMap::new(),
            functions: HashMap::new(),
            current_items: context.current_item.clone().into_iter().collect(),
            counters: HashMap::new(),
            limits: limits_from_policy(context.scope_policy),
            call_depth: 0,
            diagnostics: context.diagnostics.clone(),
            error: None,
            module_resolution: context.module_resolution.clone(),
        };
        ctx.index_bindings();
        ctx.bind_policy_bindings(context);
        ctx
    }

    fn index_bindings(&mut self) {
        for (index, node) in self.query.tree.nodes.iter().enumerate() {
            let id = IrId(index.try_into().unwrap_or(u32::MAX));
            if let Some(binding) = self.query.tree.resolutions.get(index).and_then(|r| *r) {
                match node {
                    IrNode::Lambda { .. } => {
                        self.functions.insert(binding, id);
                    }
                    IrNode::Let { name, .. } if *name == binding => {
                        self.globals.insert(binding, id);
                    }
                    _ => {}
                }
            }
        }
    }

    fn bind_policy_bindings(&mut self, context: &EvaluationContext) {
        for (binding, name) in &self.query.policy_bindings {
            if let Some(value) = context.policy_bindings.get(name).cloned() {
                self.scopes
                    .first_mut()
                    .expect("evaluator always has a root scope")
                    .insert(*binding, value);
            }
        }
    }

    pub(crate) fn eval_id(&mut self, id: IrId) -> ItemStream {
        if let Err(cancelled) = self.ensure_active(id) {
            return cancelled;
        }
        let Some(node) = self.query.tree.node(id).cloned() else {
            return self.unsupported(id, "missing IR node");
        };
        match node {
            IrNode::LitString(value) => ItemStream::once(Item::Atomic(AtomValue::String(value))),
            IrNode::LitInt(value) => ItemStream::once(Item::Atomic(AtomValue::Integer(value))),
            IrNode::LitDecimal(value) => ItemStream::once(Item::Atomic(AtomValue::Decimal(value))),
            IrNode::LitDouble(value) => ItemStream::once(Item::Atomic(AtomValue::Double(value))),
            IrNode::LitBool(value) => ItemStream::once(Item::Atomic(AtomValue::Boolean(value))),
            IrNode::LitNull => ItemStream::once(Item::Atomic(AtomValue::Null)),
            IrNode::LocalVar(binding) => self.lookup_var(binding),
            IrNode::FunctionRef(binding) => self
                .functions
                .get(&binding)
                .copied()
                .map(|lambda| ItemStream::once(Item::Lambda(lambda)))
                .unwrap_or_else(|| self.unknown_function(id, "unbound function reference")),
            IrNode::SchemaType(_) | IrNode::TemplateRef(_) => self.unsupported(
                id,
                "schema/template references are not evaluable values yet",
            ),
            IrNode::StateSlot(slot) => ItemStream::once(Item::Atomic(AtomValue::String(format!(
                "state-slot:{}",
                slot.0
            )))),
            IrNode::Record(entries) => {
                let mut record = BTreeMap::new();
                for (key, value_id) in entries {
                    if let Err(error) = self.poll_work(id) {
                        return error;
                    }
                    let key = match self.eval_record_key(id, &key) {
                        Ok(key) => key,
                        Err(stream) => return stream,
                    };
                    let stream = self.eval_id(value_id);
                    self.merge_stream_status(&stream);
                    record.insert(key, stream.items);
                }
                ItemStream::once(Item::Record(record))
            }
            IrNode::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    if let Err(error) = self.poll_work(id) {
                        return error;
                    }
                    let stream = self.eval_id(item);
                    self.merge_stream_status(&stream);
                    out.extend(stream.items);
                }
                ItemStream::once(Item::Array(out))
            }
            IrNode::Sequence(items) => {
                let mut out = ItemStream::empty();
                for item in items {
                    if let Err(error) = self.poll_work(id) {
                        return error;
                    }
                    let stream = self.eval_id(item);
                    out.append_stream(stream);
                }
                out
            }
            IrNode::Lambda { .. } => ItemStream::once(Item::Lambda(id)),
            IrNode::AxisStep { .. } | IrNode::Parent | IrNode::Self_ | IrNode::Reference => {
                self.unsupported(id, "host AST axis evaluation is not wired yet")
            }
            IrNode::Pipeline { source, steps } => {
                let source = self.eval_id(source);
                pipeline::apply_pipeline(source, &steps, self)
            }
            IrNode::LeadingDot => self
                .current_items
                .last()
                .cloned()
                .map(ItemStream::once)
                .unwrap_or_else(|| self.unsupported(id, "`.` has no current item")),
            IrNode::Call { callee, args } => self.eval_call(id, callee, &args),
            IrNode::StdlibCall { module, name, args } => {
                pipeline::apply_stdlib_call(&module, &name, &args, self)
            }
            IrNode::BinaryOp { op, lhs, rhs } => self.eval_binary(id, op, lhs, rhs),
            IrNode::UnaryOp { op, operand } => self.eval_unary(id, op, operand),
            IrNode::SetOp { op, lhs, rhs } => {
                let lhs = self.eval_id(lhs);
                let rhs = self.eval_id(rhs);
                set_ops::apply_set_op(op, lhs, rhs, self, id)
            }
            IrNode::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond = self.eval_id(cond);
                self.merge_stream_status(&cond);
                if effective_boolean(&cond.items) {
                    self.eval_id(then_branch)
                } else {
                    self.eval_id(else_branch)
                }
            }
            IrNode::Let { name, value, body } => {
                let value = self.eval_id(value);
                self.push_scope();
                self.bind(name, value.clone());
                let mut body = self.eval_id(body);
                body.extend_diagnostics(value);
                self.pop_scope();
                body
            }
            IrNode::For { var, source, body } => {
                let source = self.eval_id(source);
                self.merge_stream_status(&source);
                let mut out = ItemStream::empty();
                for item in source.items {
                    if let Err(error) = self.poll_work(id) {
                        out.append_stream(error);
                        break;
                    }
                    self.push_scope();
                    self.bind(var, ItemStream::once(item));
                    let body = self.eval_id(body);
                    out.append_stream(body);
                    self.pop_scope();
                    if out.error.is_some() {
                        break;
                    }
                }
                out
            }
            IrNode::Quantified {
                kind,
                var,
                source,
                predicate,
            } => self.eval_quantified(kind, var, source, predicate),
            IrNode::InstanceOf { value, ty } => {
                let stream = self.eval_id(value);
                let ok = stream
                    .items
                    .first()
                    .is_some_and(|item| types_runtime::item_matches_type(item, &ty));
                let mut out = ItemStream::once(Item::Atomic(AtomValue::Boolean(ok)));
                out.extend_diagnostics(stream);
                out
            }
            IrNode::CastAs { value, ty } => {
                let stream = self.eval_id(value);
                types_runtime::cast_stream(stream, &ty, self, id)
            }
            IrNode::TreatAs { value, ty } => {
                let stream = self.eval_id(value);
                types_runtime::treat_stream(stream, &ty, self, id)
            }
            IrNode::Is { lhs, rhs } => {
                let lhs = self.eval_id(lhs);
                let rhs = self.eval_id(rhs);
                let same = lhs
                    .items
                    .first()
                    .zip(rhs.items.first())
                    .is_some_and(|(lhs, rhs)| item_identity(lhs) == item_identity(rhs));
                let mut out = ItemStream::once(Item::Atomic(AtomValue::Boolean(same)));
                out.extend_diagnostics(lhs);
                out.extend_diagnostics(rhs);
                out
            }
        }
    }

    pub(crate) fn invoke_lambda(&mut self, lambda: IrId, args: Vec<ItemStream>) -> ItemStream {
        let Some(IrNode::Lambda { params, body, .. }) = self.query.tree.node(lambda).cloned()
        else {
            return self.unsupported(lambda, "call target is not a lambda");
        };
        if let Err(err) = self.enter_call(lambda) {
            return err;
        }
        self.push_scope();
        for ((binding, _), arg) in params.into_iter().zip(args) {
            self.bind(binding, arg);
        }
        let out = self.eval_id(body);
        self.pop_scope();
        self.exit_call();
        out
    }

    pub(crate) fn invoke_function(
        &mut self,
        binding: BindingId,
        args: Vec<ItemStream>,
        source: IrId,
    ) -> ItemStream {
        self.functions
            .get(&binding)
            .copied()
            .map(|lambda| self.invoke_lambda(lambda, args))
            .unwrap_or_else(|| self.unknown_function(source, "unbound function call"))
    }

    pub(crate) fn eval_arg_streams(&mut self, args: &[IrId]) -> Vec<ItemStream> {
        args.iter().map(|arg| self.eval_id(*arg)).collect()
    }

    pub(crate) fn with_current_item(
        &mut self,
        item: Item,
        f: impl FnOnce(&mut Self) -> ItemStream,
    ) -> ItemStream {
        self.current_items.push(item);
        let out = f(self);
        self.current_items.pop();
        out
    }

    pub(crate) fn charge_items(&mut self, amount: u64, source: IrId) -> Result<(), ItemStream> {
        self.charge(BudgetAxis::ItemsPerStage, amount, source)
    }

    pub(crate) fn poll_work(&mut self, source: IrId) -> Result<(), ItemStream> {
        let result = self.safe_points.poll_one();
        self.map_control_result(source, result)
    }

    pub(crate) fn force_safe_point(&mut self, source: IrId) -> Result<(), ItemStream> {
        let result = self.safe_points.force();
        self.map_control_result(source, result)
    }

    fn ensure_active(&mut self, source: IrId) -> Result<(), ItemStream> {
        self.poll_work(source)
    }

    fn map_control_result(
        &mut self,
        source: IrId,
        result: Result<(), ControlError>,
    ) -> Result<(), ItemStream> {
        let Err(control_error) = result else {
            return Ok(());
        };
        let (error, code, message, severity) = match control_error {
            ControlError::Triggered(mut failure) => {
                if failure.source_map.is_none() {
                    failure.source_map = self.query.tree.source_maps.get(source.0 as usize).cloned();
                }
                if failure.terminal_class() == ControlTerminalClass::Cancelled {
                    (
                        EvalError::Cancelled,
                        ABORTED.to_string(),
                        "CEM-QL evaluation cancelled by the host".to_owned(),
                        Severity::Info,
                    )
                } else {
                    let code = format!("cem.ql.control.{}", failure.code());
                    let message = failure.to_string();
                    (EvalError::Control(failure), code, message, Severity::Error)
                }
            }
            other => (
                EvalError::Unsupported("operation control check failed"),
                other.code().to_owned(),
                other.to_string(),
                Severity::Error,
            ),
        };
        let diagnostic = self.diagnostic(source, code, message, severity);
        self.diagnostics.push(diagnostic.clone());
        self.error = Some(error.clone());
        Err(ItemStream::failed(error, diagnostic))
    }

    pub(crate) fn unsupported(&mut self, source: IrId, message: &'static str) -> ItemStream {
        let diagnostic = self.diagnostic(source, TYPE_ERROR, message, Severity::Error);
        let error = EvalError::Unsupported(message);
        self.diagnostics.push(diagnostic.clone());
        self.error = Some(error.clone());
        ItemStream::failed(error, diagnostic)
    }

    pub(crate) fn type_error(&mut self, source: IrId, message: &'static str) -> ItemStream {
        let diagnostic = self.diagnostic(source, TYPE_ERROR, message, Severity::Error);
        let error = EvalError::TypeError(message);
        self.diagnostics.push(diagnostic.clone());
        self.error = Some(error.clone());
        ItemStream::failed(error, diagnostic)
    }

    pub(crate) fn unknown_function(&mut self, source: IrId, message: &'static str) -> ItemStream {
        let diagnostic = self.diagnostic(source, UNKNOWN_FUNCTION, message, Severity::Error);
        let error = EvalError::Unsupported(message);
        self.diagnostics.push(diagnostic.clone());
        self.error = Some(error.clone());
        ItemStream::failed(error, diagnostic)
    }

    fn unknown_variable(&mut self, source: IrId, message: &'static str) -> ItemStream {
        let diagnostic = self.diagnostic(source, UNKNOWN_VARIABLE, message, Severity::Error);
        let error = EvalError::Unsupported(message);
        self.diagnostics.push(diagnostic.clone());
        self.error = Some(error.clone());
        ItemStream::failed(error, diagnostic)
    }

    pub(crate) fn emit_diagnostic(
        &mut self,
        source: IrId,
        code: impl Into<String>,
        message: impl Into<String>,
        severity: Severity,
    ) -> ItemStream {
        let diagnostic = self.diagnostic(source, code, message, severity);
        self.diagnostics.push(diagnostic.clone());
        let mut out = ItemStream::empty();
        out.diagnostics.push(diagnostic);
        out
    }

    pub(crate) fn fail_diagnostic(
        &mut self,
        source: IrId,
        code: DiagnosticCode,
        message: impl Into<String>,
        error_message: &'static str,
    ) -> ItemStream {
        let diagnostic = self.diagnostic(source, code, message, Severity::Error);
        let error = EvalError::Unsupported(error_message);
        self.diagnostics.push(diagnostic.clone());
        self.error = Some(error.clone());
        ItemStream::failed(error, diagnostic)
    }

    pub(crate) fn module_resolution(&self) -> Option<&CemModuleUrlResolutionCapability> {
        self.module_resolution.as_ref()
    }

    pub(crate) fn current_module_resolution_context(
        &self,
        capability: &CemModuleUrlResolutionCapability,
    ) -> CemResolutionContextHandle {
        self.current_items
            .last()
            .and_then(Item::module_resolution_node_identity)
            .and_then(|identity| capability.node_context(&identity).cloned())
            .unwrap_or_else(|| capability.context().clone())
    }

    pub(crate) fn source_map(&self, source: IrId) -> SourceMapStack {
        self.query
            .tree
            .source_maps
            .get(source.0 as usize)
            .cloned()
            .unwrap_or_default()
    }

    fn eval_call(&mut self, source: IrId, callee: IrId, args: &[IrId]) -> ItemStream {
        if let Some(IrNode::FunctionRef(binding)) = self.query.tree.node(callee).cloned() {
            let args = self.eval_arg_streams(args);
            return self.invoke_function(binding, args, source);
        }
        let callee_stream = self.eval_id(callee);
        let Some(Item::Lambda(lambda)) = callee_stream.items.first().cloned() else {
            let mut out = self.type_error(source, "callee did not evaluate to a lambda");
            out.extend_diagnostics(callee_stream);
            return out;
        };
        let args = self.eval_arg_streams(args);
        let mut out = self.invoke_lambda(lambda, args);
        out.extend_diagnostics(callee_stream);
        out
    }

    fn eval_record_key(
        &mut self,
        record_source: IrId,
        key: &IrRecordKey,
    ) -> Result<String, ItemStream> {
        match key {
            IrRecordKey::Static(key) => Ok(key.clone()),
            IrRecordKey::Computed(key_id) => {
                let stream = self.eval_id(*key_id);
                self.merge_stream_status(&stream);
                match stream.items.as_slice() {
                    [Item::Atomic(AtomValue::String(value))] => Ok(value.clone()),
                    [item] if matches!(item.atom(), Some(AtomValue::String(_))) => {
                        let Some(AtomValue::String(value)) = item.atom() else {
                            unreachable!();
                        };
                        Ok(value)
                    }
                    _ => Err(self.type_error(
                        record_source,
                        "computed record key must evaluate to exactly one string",
                    )),
                }
            }
        }
    }

    fn eval_binary(&mut self, source: IrId, op: BinaryOp, lhs: IrId, rhs: IrId) -> ItemStream {
        if op == BinaryOp::Coalesce {
            // Null/empty-sequence coalescing: short-circuit to the left operand unless
            // it is empty or its first item is `null`, otherwise evaluate the right.
            let lhs = self.eval_id(lhs);
            self.merge_stream_status(&lhs);
            if lhs.error.is_some() {
                return lhs;
            }
            let present = lhs
                .items
                .first()
                .is_some_and(|item| !matches!(item.atom(), Some(AtomValue::Null)));
            if present {
                return lhs;
            }
            let mut out = self.eval_id(rhs);
            out.extend_diagnostics(lhs);
            return out;
        }
        if op == BinaryOp::And {
            let lhs = self.eval_id(lhs);
            self.merge_stream_status(&lhs);
            if !effective_boolean(&lhs.items) {
                return ItemStream::once(Item::Atomic(AtomValue::Boolean(false)));
            }
            let rhs = self.eval_id(rhs);
            let ok = effective_boolean(&rhs.items);
            let mut out = ItemStream::once(Item::Atomic(AtomValue::Boolean(ok)));
            out.extend_diagnostics(lhs);
            out.extend_diagnostics(rhs);
            return out;
        }
        if op == BinaryOp::Or {
            let lhs = self.eval_id(lhs);
            self.merge_stream_status(&lhs);
            if effective_boolean(&lhs.items) {
                return ItemStream::once(Item::Atomic(AtomValue::Boolean(true)));
            }
            let rhs = self.eval_id(rhs);
            let ok = effective_boolean(&rhs.items);
            let mut out = ItemStream::once(Item::Atomic(AtomValue::Boolean(ok)));
            out.extend_diagnostics(lhs);
            out.extend_diagnostics(rhs);
            return out;
        }

        let lhs_stream = self.eval_id(lhs);
        let rhs_stream = self.eval_id(rhs);
        if op == BinaryOp::Minus {
            return self.eval_runtime_minus(source, lhs_stream, rhs_stream);
        }
        let lhs = lhs_stream.items.first();
        let rhs = rhs_stream.items.first();
        let item = match (op, lhs, rhs) {
            (BinaryOp::Plus, Some(lhs), Some(rhs)) => match plus_binary(lhs, rhs) {
                Ok(item) => item,
                Err(message) => {
                    let mut out = self.type_error(source, message);
                    out.extend_diagnostics(lhs_stream);
                    out.extend_diagnostics(rhs_stream);
                    return out;
                }
            },
            (op @ (BinaryOp::Star | BinaryOp::Slash | BinaryOp::Percent), Some(lhs), Some(rhs)) => {
                match numeric_binary(op, lhs, rhs) {
                    Ok(item) => item,
                    Err(message) => {
                        let mut out = self.type_error(source, message);
                        out.extend_diagnostics(lhs_stream);
                        out.extend_diagnostics(rhs_stream);
                        return out;
                    }
                }
            }
            (BinaryOp::EqEq, Some(lhs), Some(rhs)) => Item::Atomic(AtomValue::Boolean(
                atom_cmp(lhs, rhs) == Some(std::cmp::Ordering::Equal),
            )),
            (BinaryOp::BangEq, Some(lhs), Some(rhs)) => Item::Atomic(AtomValue::Boolean(
                atom_cmp(lhs, rhs) != Some(std::cmp::Ordering::Equal),
            )),
            (BinaryOp::Lt, Some(lhs), Some(rhs)) => Item::Atomic(AtomValue::Boolean(
                atom_cmp(lhs, rhs) == Some(std::cmp::Ordering::Less),
            )),
            (BinaryOp::Le, Some(lhs), Some(rhs)) => Item::Atomic(AtomValue::Boolean(matches!(
                atom_cmp(lhs, rhs),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ))),
            (BinaryOp::Gt, Some(lhs), Some(rhs)) => Item::Atomic(AtomValue::Boolean(
                atom_cmp(lhs, rhs) == Some(std::cmp::Ordering::Greater),
            )),
            (BinaryOp::Ge, Some(lhs), Some(rhs)) => Item::Atomic(AtomValue::Boolean(matches!(
                atom_cmp(lhs, rhs),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ))),
            _ => {
                return self.type_error(source, runtime_binary_operand_message(op));
            }
        };
        let mut out = ItemStream::once(item);
        out.extend_diagnostics(lhs_stream);
        out.extend_diagnostics(rhs_stream);
        out
    }

    fn eval_runtime_minus(
        &mut self,
        source: IrId,
        lhs_stream: ItemStream,
        rhs_stream: ItemStream,
    ) -> ItemStream {
        match (
            single_numeric_item(&lhs_stream.items),
            single_numeric_item(&rhs_stream.items),
        ) {
            (Some(lhs), Some(rhs)) => match numeric_binary(BinaryOp::Minus, lhs, rhs) {
                Ok(item) => {
                    let mut out = ItemStream::once(item);
                    out.extend_diagnostics(lhs_stream);
                    out.extend_diagnostics(rhs_stream);
                    out
                }
                Err(message) => {
                    let mut out = self.type_error(source, message);
                    out.extend_diagnostics(lhs_stream);
                    out.extend_diagnostics(rhs_stream);
                    out
                }
            },
            (Some(_), None) | (None, Some(_)) => {
                let mut out = self.type_error(
                    source,
                    "operator `-` cannot mix numeric and stream operands",
                );
                out.extend_diagnostics(lhs_stream);
                out.extend_diagnostics(rhs_stream);
                out
            }
            (None, None) => {
                set_ops::apply_set_op(SetOp::Difference, lhs_stream, rhs_stream, self, source)
            }
        }
    }

    fn eval_unary(&mut self, source: IrId, op: UnaryOp, operand: IrId) -> ItemStream {
        let operand_stream = self.eval_id(operand);
        let item = match op {
            UnaryOp::Not => Item::Atomic(AtomValue::Boolean(!effective_boolean(
                &operand_stream.items,
            ))),
            UnaryOp::Negate => {
                let Some(item) = operand_stream.items.first() else {
                    return self.type_error(source, "unary minus requires a numeric item");
                };
                match numeric_unary(item) {
                    Ok(item) => item,
                    Err(message) => {
                        let mut out = self.type_error(source, message);
                        out.extend_diagnostics(operand_stream);
                        return out;
                    }
                }
            }
        };
        let mut out = ItemStream::once(item);
        out.extend_diagnostics(operand_stream);
        out
    }

    fn eval_quantified(
        &mut self,
        kind: QuantifierKind,
        var: BindingId,
        source: IrId,
        predicate: IrId,
    ) -> ItemStream {
        let source = self.eval_id(source);
        self.merge_stream_status(&source);
        let mut any = false;
        for item in source.items {
            if let Err(error) = self.poll_work(predicate) {
                return error;
            }
            self.push_scope();
            self.bind(var, ItemStream::once(item));
            let predicate = self.eval_id(predicate);
            let passed = effective_boolean(&predicate.items);
            self.pop_scope();
            match kind {
                QuantifierKind::Some if passed => {
                    any = true;
                    break;
                }
                QuantifierKind::Every if !passed => {
                    return ItemStream::once(Item::Atomic(AtomValue::Boolean(false)));
                }
                _ => {}
            }
        }
        let value = match kind {
            QuantifierKind::Some => any,
            QuantifierKind::Every => true,
        };
        ItemStream::once(Item::Atomic(AtomValue::Boolean(value)))
    }

    fn lookup_var(&mut self, binding: BindingId) -> ItemStream {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(&binding) {
                return value.clone();
            }
        }
        if let Some(let_id) = self.globals.get(&binding).copied() {
            if let Some(IrNode::Let { value, .. }) = self.query.tree.node(let_id).cloned() {
                return self.eval_id(value);
            }
        }
        self.unknown_variable(self.query.tree.root, "unbound local variable")
    }

    fn bind(&mut self, binding: BindingId, value: ItemStream) {
        self.scopes
            .last_mut()
            .expect("evaluator always has a scope")
            .insert(binding, value);
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn enter_call(&mut self, source: IrId) -> Result<(), ItemStream> {
        self.force_safe_point(source)?;
        self.charge(BudgetAxis::FunctionCalls, 1, source)?;
        self.call_depth += 1;
        if let Err(err) = self.charge(BudgetAxis::CallDepth, 1, source) {
            self.call_depth = self.call_depth.saturating_sub(1);
            return Err(err);
        }
        Ok(())
    }

    fn exit_call(&mut self) {
        self.call_depth = self.call_depth.saturating_sub(1);
    }

    fn charge(&mut self, axis: BudgetAxis, amount: u64, source: IrId) -> Result<(), ItemStream> {
        self.ensure_active(source)?;
        let current = self.counters.get(&axis).copied().unwrap_or(0);
        let next = current.saturating_add(amount);
        let limit = self.limits.get(&axis).copied().unwrap_or(u64::MAX);
        if next > limit {
            let message = format!("cem-ql budget exceeded: {}", axis.as_str());
            let diagnostic = self.diagnostic(source, BUDGET_EXCEEDED, message, Severity::Error);
            let error = EvalError::BudgetExceeded(axis);
            self.diagnostics.push(diagnostic.clone());
            self.error = Some(error.clone());
            return Err(ItemStream::failed(error, diagnostic));
        }
        self.counters.insert(axis, next);
        Ok(())
    }

    fn merge_stream_status(&mut self, stream: &ItemStream) {
        self.diagnostics.extend(stream.diagnostics.clone());
        if self.error.is_none() {
            self.error = stream.error.clone();
        }
    }

    fn diagnostic(
        &self,
        source: IrId,
        code: impl Into<String>,
        message: impl Into<String>,
        severity: Severity,
    ) -> Diagnostic {
        let source_map = self.query.tree.source_maps.get(source.0 as usize).cloned();
        let byte_offset = source_map
            .as_ref()
            .and_then(SourceMapStack::current)
            .map(|frame| match frame.span {
                cem_ml::source_map::FrameSpan::Single(range) => range.start,
                cem_ml::source_map::FrameSpan::Multi(ref ranges) => {
                    ranges.first().map(|range| range.start).unwrap_or(0)
                }
            });
        Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset,
            code: code.into(),
            severity,
            message: message.into(),
            node: None,
            details: None,
            source_map,
        }
    }
}

pub(crate) fn item_identity(item: &Item) -> String {
    match item {
        Item::Node(id) => format!("node:{id}"),
        Item::Atomic(atom) => format!("atom:{}", atom_identity(atom)),
        Item::Record(entries) => format!("record:{entries:?}"),
        Item::Array(items) => format!("array:{items:?}"),
        Item::Native(view) => format!("native:{}:{}", view.representation_id(), view.identity()),
        Item::Lambda(id) => format!("lambda:{}", id.0),
        Item::Resource(handle) => format!("resource:{}:{}", handle.content_type, handle.id),
    }
}

pub(crate) fn effective_boolean(items: &[Item]) -> bool {
    let Some(first) = items.first() else {
        return false;
    };
    match first.atom() {
        Some(AtomValue::Boolean(value)) => value,
        Some(AtomValue::Integer(value)) => value != 0,
        Some(AtomValue::Decimal(value)) => value != "0" && value != "0.0",
        Some(AtomValue::Double(value)) => value != 0.0 && !value.is_nan(),
        Some(AtomValue::String(value)) | Some(AtomValue::AnyUri(value)) => !value.is_empty(),
        Some(AtomValue::Null) => false,
        _ => true,
    }
}

pub(crate) fn first_integer(stream: &ItemStream) -> Option<i64> {
    stream.items.first().and_then(item_to_i64)
}

fn limits_from_policy(policy: ScopePolicy) -> HashMap<BudgetAxis, u64> {
    [
        (BudgetAxis::ItemsPerStage, policy.queue_size.max(1) as u64),
        (
            BudgetAxis::CallDepth,
            (policy.cpu_workers.max(1) as u64) * 16,
        ),
        (
            BudgetAxis::FunctionCalls,
            (policy.queue_size.max(1) as u64) * 16,
        ),
        (BudgetAxis::ClosureSize, policy.memory_bytes.max(1)),
        (BudgetAxis::RegexBacktrack, u64::MAX),
        (BudgetAxis::ExternalFetches, policy.io_streams.max(1) as u64),
    ]
    .into_iter()
    .collect()
}

fn atom_identity(atom: &AtomValue) -> String {
    match atom {
        AtomValue::String(value) => format!("string:{value}"),
        AtomValue::Integer(value) => format!("integer:{value}"),
        AtomValue::Decimal(value) => format!("decimal:{value}"),
        AtomValue::Double(value) if value.is_nan() => "double:NaN".to_owned(),
        AtomValue::Double(value) => format!("double:{:x}", value.to_bits()),
        AtomValue::Boolean(value) => format!("boolean:{value}"),
        AtomValue::AnyUri(value) => format!("any-uri:{value}"),
        AtomValue::Null => "null".to_owned(),
    }
}

fn item_to_f64(item: &Item) -> Option<f64> {
    match item.atom()? {
        AtomValue::Integer(value) => Some(value as f64),
        AtomValue::Decimal(value) => value.parse().ok(),
        AtomValue::Double(value) => Some(value),
        AtomValue::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn item_to_i64(item: &Item) -> Option<i64> {
    match item.atom()? {
        AtomValue::Integer(value) => Some(value),
        AtomValue::Decimal(value) => value.parse().ok(),
        AtomValue::Double(value) => Some(value as i64),
        AtomValue::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn item_to_string(item: &Item) -> Option<String> {
    if let Some(atom) = item.atom() {
        return match atom {
            AtomValue::String(value) | AtomValue::AnyUri(value) => Some(value),
            AtomValue::Integer(value) => Some(value.to_string()),
            AtomValue::Decimal(value) => Some(value),
            AtomValue::Double(value) => Some(value.to_string()),
            AtomValue::Boolean(value) => Some(value.to_string()),
            AtomValue::Null => Some("null".to_owned()),
        };
    }
    match item {
        Item::Node(id) => Some(id.clone()),
        _ => None,
    }
}

fn numeric_binary(op: BinaryOp, lhs: &Item, rhs: &Item) -> Result<Item, &'static str> {
    match (lhs.atom(), rhs.atom()) {
        (Some(AtomValue::Integer(lhs)), Some(AtomValue::Integer(rhs))) => {
            integer_binary(op, lhs, rhs).map(|value| Item::Atomic(AtomValue::Integer(value)))
        }
        (Some(AtomValue::Decimal(lhs)), Some(AtomValue::Decimal(rhs))) => {
            decimal_binary(op, &lhs, &rhs).map(|value| Item::Atomic(AtomValue::Decimal(value)))
        }
        (Some(AtomValue::Double(lhs)), Some(AtomValue::Double(rhs))) => {
            Ok(Item::Atomic(AtomValue::Double(double_binary(op, lhs, rhs))))
        }
        _ if is_numeric_item(lhs) && is_numeric_item(rhs) => Err(runtime_mixed_numeric_message(op)),
        _ => Err(runtime_binary_operand_message(op)),
    }
}

fn plus_binary(lhs: &Item, rhs: &Item) -> Result<Item, &'static str> {
    match (lhs.atom(), rhs.atom()) {
        (Some(AtomValue::String(lhs)), Some(AtomValue::String(rhs))) => {
            Ok(Item::Atomic(AtomValue::String(lhs + &rhs)))
        }
        _ => numeric_binary(BinaryOp::Plus, lhs, rhs),
    }
}

fn integer_binary(op: BinaryOp, lhs: i64, rhs: i64) -> Result<i64, &'static str> {
    match op {
        BinaryOp::Plus => lhs
            .checked_add(rhs)
            .ok_or("operator `+` overflowed integer"),
        BinaryOp::Minus => lhs
            .checked_sub(rhs)
            .ok_or("operator `-` overflowed integer"),
        BinaryOp::Star => lhs
            .checked_mul(rhs)
            .ok_or("operator `*` overflowed integer"),
        BinaryOp::Slash if rhs == 0 => Err("operator `/` cannot divide integer by zero"),
        BinaryOp::Slash => lhs
            .checked_div(rhs)
            .ok_or("operator `/` overflowed integer"),
        BinaryOp::Percent if rhs == 0 => Err("operator `%` cannot divide integer by zero"),
        BinaryOp::Percent => lhs
            .checked_rem(rhs)
            .ok_or("operator `%` overflowed integer"),
        _ => Err(runtime_binary_operand_message(op)),
    }
}

fn decimal_binary(op: BinaryOp, lhs: &str, rhs: &str) -> Result<String, &'static str> {
    let lhs = DecimalParts::parse(lhs).ok_or(decimal_operand_message(op))?;
    let rhs = DecimalParts::parse(rhs).ok_or(decimal_operand_message(op))?;
    let value = match op {
        BinaryOp::Plus => lhs
            .checked_add(rhs)
            .ok_or("operator `+` overflowed decimal")?,
        BinaryOp::Minus => lhs
            .checked_sub(rhs)
            .ok_or("operator `-` overflowed decimal")?,
        BinaryOp::Star => lhs
            .checked_mul(rhs)
            .ok_or("operator `*` overflowed decimal")?,
        BinaryOp::Slash => lhs.checked_div(rhs)?,
        BinaryOp::Percent => lhs.checked_rem(rhs)?,
        _ => return Err(runtime_binary_operand_message(op)),
    };
    Ok(value.format())
}

fn double_binary(op: BinaryOp, lhs: f64, rhs: f64) -> f64 {
    match op {
        BinaryOp::Plus => lhs + rhs,
        BinaryOp::Minus => lhs - rhs,
        BinaryOp::Star => lhs * rhs,
        BinaryOp::Slash => lhs / rhs,
        BinaryOp::Percent => lhs % rhs,
        _ => f64::NAN,
    }
}

fn is_numeric_item(item: &Item) -> bool {
    matches!(
        item.atom(),
        Some(AtomValue::Integer(_) | AtomValue::Decimal(_) | AtomValue::Double(_))
    )
}

fn runtime_mixed_numeric_message(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Plus => {
            "operator `+` requires matching numeric operand types; use an explicit num:* conversion"
        }
        BinaryOp::Minus => {
            "operator `-` requires matching numeric operand types; use an explicit num:* conversion"
        }
        BinaryOp::Star => {
            "operator `*` requires matching numeric operand types; use an explicit num:* conversion"
        }
        BinaryOp::Slash => {
            "operator `/` requires matching numeric operand types; use an explicit num:* conversion"
        }
        BinaryOp::Percent => {
            "operator `%` requires matching numeric operand types; use an explicit num:* conversion"
        }
        _ => runtime_binary_operand_message(op),
    }
}

fn decimal_operand_message(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Plus => "operator `+` requires finite decimal operands",
        BinaryOp::Minus => "operator `-` requires finite decimal operands",
        BinaryOp::Star => "operator `*` requires finite decimal operands",
        BinaryOp::Slash => "operator `/` requires finite decimal operands",
        BinaryOp::Percent => "operator `%` requires finite decimal operands",
        _ => runtime_binary_operand_message(op),
    }
}

fn single_numeric_item(items: &[Item]) -> Option<&Item> {
    match items {
        [item] if is_numeric_item(item) => Some(item),
        _ => None,
    }
}

fn runtime_binary_operand_message(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::EqEq => "operator `==` operands are not supported",
        BinaryOp::BangEq => "operator `!=` operands are not supported",
        BinaryOp::Lt => "operator `<` operands are not supported",
        BinaryOp::Le => "operator `<=` operands are not supported",
        BinaryOp::Gt => "operator `>` operands are not supported",
        BinaryOp::Ge => "operator `>=` operands are not supported",
        BinaryOp::Plus => {
            "operator `+` requires two strings or matching numeric operands; implicit string conversion is not supported"
        }
        BinaryOp::Minus => "operator `-` requires numeric or stream operands",
        BinaryOp::Star => "operator `*` requires numeric operands",
        BinaryOp::Slash => "operator `/` requires numeric operands",
        BinaryOp::Percent => "operator `%` requires numeric operands",
        BinaryOp::And => "operator `&&` requires boolean-compatible operands",
        BinaryOp::Or => "operator `||` requires boolean-compatible operands",
        BinaryOp::Coalesce => "operator `??` operands are not supported",
        BinaryOp::Is => "operator `is` requires node operands",
    }
}

fn numeric_unary(item: &Item) -> Result<Item, &'static str> {
    match item.atom() {
        Some(AtomValue::Integer(value)) => value
            .checked_neg()
            .map(|value| Item::Atomic(AtomValue::Integer(value)))
            .ok_or("unary minus overflowed integer"),
        Some(AtomValue::Decimal(value)) => {
            let decimal = DecimalParts::parse(&value)
                .ok_or("unary minus requires a finite decimal operand")?;
            decimal
                .checked_neg()
                .map(|value| Item::Atomic(AtomValue::Decimal(value.format())))
                .ok_or("unary minus overflowed decimal")
        }
        Some(AtomValue::Double(value)) => Ok(Item::Atomic(AtomValue::Double(-value))),
        _ => Err("unary minus requires a numeric item"),
    }
}

#[derive(Clone, Copy)]
struct DecimalParts {
    units: i128,
    scale: u32,
}

impl DecimalParts {
    fn parse(source: &str) -> Option<Self> {
        let source = source.trim();
        let (negative, source) = source
            .strip_prefix('-')
            .map(|source| (true, source))
            .or_else(|| source.strip_prefix('+').map(|source| (false, source)))
            .unwrap_or((false, source));
        if source.is_empty() || source.contains('e') || source.contains('E') {
            return None;
        }
        let (whole, fraction) = source.split_once('.').unwrap_or((source, ""));
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        if !whole.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let scale = u32::try_from(fraction.len()).ok()?;
        let mut units = 0i128;
        for byte in whole.bytes().chain(fraction.bytes()) {
            units = units.checked_mul(10)?;
            units = units.checked_add(i128::from(byte - b'0'))?;
        }
        if negative {
            units = units.checked_neg()?;
        }
        Some(Self { units, scale }.normalized())
    }

    fn checked_neg(self) -> Option<Self> {
        Some(
            Self {
                units: self.units.checked_neg()?,
                scale: self.scale,
            }
            .normalized(),
        )
    }

    fn checked_add(self, rhs: Self) -> Option<Self> {
        let (lhs_units, rhs_units, scale) = align_decimal_scales(self, rhs)?;
        Some(
            Self {
                units: lhs_units.checked_add(rhs_units)?,
                scale,
            }
            .normalized(),
        )
    }

    fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (lhs_units, rhs_units, scale) = align_decimal_scales(self, rhs)?;
        Some(
            Self {
                units: lhs_units.checked_sub(rhs_units)?,
                scale,
            }
            .normalized(),
        )
    }

    fn checked_mul(self, rhs: Self) -> Option<Self> {
        Some(
            Self {
                units: self.units.checked_mul(rhs.units)?,
                scale: self.scale.checked_add(rhs.scale)?,
            }
            .normalized(),
        )
    }

    fn checked_div(self, rhs: Self) -> Result<Self, &'static str> {
        if rhs.units == 0 {
            return Err("operator `/` cannot divide decimal by zero");
        }
        let mut numerator = self
            .units
            .checked_mul(pow10_i128(rhs.scale).ok_or("operator `/` overflowed decimal")?)
            .ok_or("operator `/` overflowed decimal")?;
        let mut denominator = rhs
            .units
            .checked_mul(pow10_i128(self.scale).ok_or("operator `/` overflowed decimal")?)
            .ok_or("operator `/` overflowed decimal")?;
        if denominator < 0 {
            numerator = numerator
                .checked_neg()
                .ok_or("operator `/` overflowed decimal")?;
            denominator = denominator
                .checked_neg()
                .ok_or("operator `/` overflowed decimal")?;
        }
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator as u128) as i128;
        numerator /= divisor;
        denominator /= divisor;

        let mut reduced_denominator = denominator;
        let mut twos = 0u32;
        while reduced_denominator % 2 == 0 {
            reduced_denominator /= 2;
            twos += 1;
        }
        let mut fives = 0u32;
        while reduced_denominator % 5 == 0 {
            reduced_denominator /= 5;
            fives += 1;
        }
        if reduced_denominator != 1 {
            return Err(
                "operator `/` decimal result is not finite; use num:double(...) for IEEE division",
            );
        }

        let scale = twos.max(fives);
        let mut units = numerator;
        for _ in 0..(scale - twos) {
            units = units
                .checked_mul(2)
                .ok_or("operator `/` overflowed decimal")?;
        }
        for _ in 0..(scale - fives) {
            units = units
                .checked_mul(5)
                .ok_or("operator `/` overflowed decimal")?;
        }
        Ok(Self { units, scale }.normalized())
    }

    fn checked_rem(self, rhs: Self) -> Result<Self, &'static str> {
        let (lhs_units, rhs_units, scale) =
            align_decimal_scales(self, rhs).ok_or("operator `%` overflowed decimal")?;
        if rhs_units == 0 {
            return Err("operator `%` cannot divide decimal by zero");
        }
        Ok(Self {
            units: lhs_units
                .checked_rem(rhs_units)
                .ok_or("operator `%` overflowed decimal")?,
            scale,
        }
        .normalized())
    }

    fn normalized(mut self) -> Self {
        if self.units == 0 {
            self.scale = 0;
            return self;
        }
        while self.scale > 0 && self.units % 10 == 0 {
            self.units /= 10;
            self.scale -= 1;
        }
        self
    }

    fn format(self) -> String {
        if self.scale == 0 {
            return self.units.to_string();
        }
        let negative = self.units < 0;
        let mut digits = self.units.unsigned_abs().to_string();
        let scale = self.scale as usize;
        if digits.len() <= scale {
            let mut padded = String::with_capacity(scale + 1);
            padded.push_str(&"0".repeat(scale + 1 - digits.len()));
            padded.push_str(&digits);
            digits = padded;
        }
        let split = digits.len() - scale;
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push_str(&digits[..split]);
        out.push('.');
        out.push_str(&digits[split..]);
        out
    }
}

fn align_decimal_scales(lhs: DecimalParts, rhs: DecimalParts) -> Option<(i128, i128, u32)> {
    let scale = lhs.scale.max(rhs.scale);
    let lhs_units = lhs.units.checked_mul(pow10_i128(scale - lhs.scale)?)?;
    let rhs_units = rhs.units.checked_mul(pow10_i128(scale - rhs.scale)?)?;
    Some((lhs_units, rhs_units, scale))
}

fn pow10_i128(exp: u32) -> Option<i128> {
    let mut value = 1i128;
    for _ in 0..exp {
        value = value.checked_mul(10)?;
    }
    Some(value)
}

fn gcd_u128(mut lhs: u128, mut rhs: u128) -> u128 {
    while rhs != 0 {
        let remainder = lhs % rhs;
        lhs = rhs;
        rhs = remainder;
    }
    lhs.max(1)
}

fn atom_cmp(lhs: &Item, rhs: &Item) -> Option<std::cmp::Ordering> {
    match (lhs.atom(), rhs.atom()) {
        (Some(lhs), Some(rhs)) => atom_value_cmp(&lhs, &rhs),
        _ => match (lhs, rhs) {
            (Item::Node(lhs), Item::Node(rhs)) => lhs.partial_cmp(rhs),
            _ => None,
        },
    }
}

fn atom_value_cmp(lhs: &AtomValue, rhs: &AtomValue) -> Option<std::cmp::Ordering> {
    match (lhs, rhs) {
        (AtomValue::String(lhs), AtomValue::String(rhs))
        | (AtomValue::AnyUri(lhs), AtomValue::AnyUri(rhs)) => lhs.partial_cmp(rhs),
        (AtomValue::Integer(lhs), AtomValue::Integer(rhs)) => lhs.partial_cmp(rhs),
        (AtomValue::Decimal(lhs), AtomValue::Decimal(rhs)) => {
            let lhs = lhs.parse::<f64>().ok()?;
            let rhs = rhs.parse::<f64>().ok()?;
            lhs.partial_cmp(&rhs)
        }
        (AtomValue::Double(lhs), AtomValue::Double(rhs)) => lhs.partial_cmp(rhs),
        (AtomValue::Boolean(lhs), AtomValue::Boolean(rhs)) => lhs.partial_cmp(rhs),
        (AtomValue::Null, AtomValue::Null) => Some(std::cmp::Ordering::Equal),
        _ => None,
    }
}

pub(crate) fn cast_item(item: &Item, ty: &Type) -> Option<Item> {
    match ty {
        Type::Atom(crate::types::AtomType::String) => {
            item_to_string(item).map(|value| Item::Atomic(AtomValue::String(value)))
        }
        Type::Atom(crate::types::AtomType::Integer) => {
            item_to_i64(item).map(|value| Item::Atomic(AtomValue::Integer(value)))
        }
        Type::Atom(crate::types::AtomType::Decimal) => {
            item_to_f64(item).map(|value| Item::Atomic(AtomValue::Decimal(value.to_string())))
        }
        Type::Atom(crate::types::AtomType::Double) => {
            item_to_f64(item).map(|value| Item::Atomic(AtomValue::Double(value)))
        }
        Type::Atom(crate::types::AtomType::Boolean) => Some(Item::Atomic(AtomValue::Boolean(
            effective_boolean(std::slice::from_ref(item)),
        ))),
        Type::Atom(crate::types::AtomType::AnyUri) => {
            item_to_string(item).map(|value| Item::Atomic(AtomValue::AnyUri(value)))
        }
        _ => Some(item.clone()).filter(|item| types_runtime::item_matches_type(item, ty)),
    }
}
