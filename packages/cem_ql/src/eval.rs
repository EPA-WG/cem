//! Layer 6: pull-based evaluator.

use std::collections::{BTreeMap, HashMap};

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::scheduler::ScopePolicy;
use cem_ml::source_map::SourceMapStack;

use crate::api::EvaluationContext;
use crate::diagnostics::{BUDGET_EXCEEDED, TYPE_ERROR, UNKNOWN_FUNCTION, UNKNOWN_VARIABLE};
use crate::ir::{CompiledQuery, IrId, IrNode};
use crate::parser::{BinaryOp, QuantifierKind, UnaryOp};
use crate::resolve::BindingId;
use crate::types::Type;

pub mod pipeline;
pub mod set_ops;
pub mod types_runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryContextScope(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Node(String),
    Atomic(AtomValue),
    Record(BTreeMap<String, Vec<Item>>),
    Array(Vec<Item>),
    Lambda(IrId),
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
    BudgetExceeded(BudgetAxis),
    Unsupported(&'static str),
    TypeError(&'static str),
}

#[derive(Debug, Clone, Default)]
pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(query: &CompiledQuery, context: &EvaluationContext) -> ItemStream {
        let mut ctx = EvalCtx::new(query, context);
        let stream = ctx.eval_id(query.tree.root);
        stream.with_context(&ctx)
    }
}

pub(crate) struct EvalCtx<'a> {
    query: &'a CompiledQuery,
    scopes: Vec<HashMap<BindingId, ItemStream>>,
    globals: HashMap<BindingId, IrId>,
    functions: HashMap<BindingId, IrId>,
    current_items: Vec<Item>,
    counters: HashMap<BudgetAxis, u64>,
    limits: HashMap<BudgetAxis, u64>,
    call_depth: u64,
    diagnostics: Vec<Diagnostic>,
    error: Option<EvalError>,
}

impl<'a> EvalCtx<'a> {
    fn new(query: &'a CompiledQuery, context: &EvaluationContext) -> Self {
        let mut ctx = Self {
            query,
            scopes: vec![HashMap::new()],
            globals: HashMap::new(),
            functions: HashMap::new(),
            current_items: Vec::new(),
            counters: HashMap::new(),
            limits: limits_from_policy(context.scope_policy),
            call_depth: 0,
            diagnostics: context.diagnostics.clone(),
            error: None,
        };
        ctx.index_bindings();
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

    pub(crate) fn eval_id(&mut self, id: IrId) -> ItemStream {
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
                    let stream = self.eval_id(value_id);
                    self.merge_stream_status(&stream);
                    record.insert(key, stream.items);
                }
                ItemStream::once(Item::Record(record))
            }
            IrNode::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    let stream = self.eval_id(item);
                    self.merge_stream_status(&stream);
                    out.extend(stream.items);
                }
                ItemStream::once(Item::Array(out))
            }
            IrNode::Sequence(items) => {
                let mut out = ItemStream::empty();
                for item in items {
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

    fn eval_binary(&mut self, source: IrId, op: BinaryOp, lhs: IrId, rhs: IrId) -> ItemStream {
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
        let lhs = lhs_stream.items.first();
        let rhs = rhs_stream.items.first();
        let item = match (op, lhs, rhs) {
            (BinaryOp::Plus, Some(lhs), Some(rhs)) => numeric_binary(lhs, rhs, |a, b| a + b),
            (BinaryOp::Minus, Some(lhs), Some(rhs)) => numeric_binary(lhs, rhs, |a, b| a - b),
            (BinaryOp::Star, Some(lhs), Some(rhs)) => numeric_binary(lhs, rhs, |a, b| a * b),
            (BinaryOp::Div, Some(lhs), Some(rhs)) => numeric_binary(lhs, rhs, |a, b| a / b),
            (BinaryOp::Mod, Some(lhs), Some(rhs)) => numeric_binary(lhs, rhs, |a, b| a % b),
            (BinaryOp::Eq | BinaryOp::EqOp, Some(lhs), Some(rhs)) => Item::Atomic(
                AtomValue::Boolean(atom_cmp(lhs, rhs) == Some(std::cmp::Ordering::Equal)),
            ),
            (BinaryOp::Ne | BinaryOp::NeqOp, Some(lhs), Some(rhs)) => Item::Atomic(
                AtomValue::Boolean(atom_cmp(lhs, rhs) != Some(std::cmp::Ordering::Equal)),
            ),
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
            _ => return self.type_error(source, "binary operator operands are not supported"),
        };
        let mut out = ItemStream::once(item);
        out.extend_diagnostics(lhs_stream);
        out.extend_diagnostics(rhs_stream);
        out
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
                numeric_unary(item, |value| -value)
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
        Item::Lambda(id) => format!("lambda:{}", id.0),
    }
}

pub(crate) fn effective_boolean(items: &[Item]) -> bool {
    let Some(first) = items.first() else {
        return false;
    };
    match first {
        Item::Atomic(AtomValue::Boolean(value)) => *value,
        Item::Atomic(AtomValue::Integer(value)) => *value != 0,
        Item::Atomic(AtomValue::Decimal(value)) => value != "0" && value != "0.0",
        Item::Atomic(AtomValue::Double(value)) => *value != 0.0 && !value.is_nan(),
        Item::Atomic(AtomValue::String(value)) | Item::Atomic(AtomValue::AnyUri(value)) => {
            !value.is_empty()
        }
        Item::Atomic(AtomValue::Null) => false,
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
    match item {
        Item::Atomic(AtomValue::Integer(value)) => Some(*value as f64),
        Item::Atomic(AtomValue::Decimal(value)) => value.parse().ok(),
        Item::Atomic(AtomValue::Double(value)) => Some(*value),
        Item::Atomic(AtomValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn item_to_i64(item: &Item) -> Option<i64> {
    match item {
        Item::Atomic(AtomValue::Integer(value)) => Some(*value),
        Item::Atomic(AtomValue::Decimal(value)) => value.parse().ok(),
        Item::Atomic(AtomValue::Double(value)) => Some(*value as i64),
        Item::Atomic(AtomValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn item_to_string(item: &Item) -> Option<String> {
    match item {
        Item::Atomic(AtomValue::String(value)) | Item::Atomic(AtomValue::AnyUri(value)) => {
            Some(value.clone())
        }
        Item::Atomic(AtomValue::Integer(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Decimal(value)) => Some(value.clone()),
        Item::Atomic(AtomValue::Double(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Boolean(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Null) => Some("null".to_owned()),
        Item::Node(id) => Some(id.clone()),
        _ => None,
    }
}

fn numeric_binary(lhs: &Item, rhs: &Item, f: impl FnOnce(f64, f64) -> f64) -> Item {
    let left = item_to_f64(lhs).unwrap_or(0.0);
    let right = item_to_f64(rhs).unwrap_or(0.0);
    let value = f(left, right);
    if matches!(lhs, Item::Atomic(AtomValue::Integer(_)))
        && matches!(rhs, Item::Atomic(AtomValue::Integer(_)))
        && value.fract() == 0.0
    {
        Item::Atomic(AtomValue::Integer(value as i64))
    } else {
        Item::Atomic(AtomValue::Double(value))
    }
}

fn numeric_unary(item: &Item, f: impl FnOnce(f64) -> f64) -> Item {
    let value = f(item_to_f64(item).unwrap_or(0.0));
    if matches!(item, Item::Atomic(AtomValue::Integer(_))) && value.fract() == 0.0 {
        Item::Atomic(AtomValue::Integer(value as i64))
    } else {
        Item::Atomic(AtomValue::Double(value))
    }
}

fn atom_cmp(lhs: &Item, rhs: &Item) -> Option<std::cmp::Ordering> {
    match (lhs, rhs) {
        (Item::Atomic(lhs), Item::Atomic(rhs)) => atom_value_cmp(lhs, rhs),
        (Item::Node(lhs), Item::Node(rhs)) => lhs.partial_cmp(rhs),
        _ => None,
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
