//! Surface AST to typed IR lowering.

use std::collections::{BTreeSet, HashMap};

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};

use crate::diagnostics::{
    DiagnosticCode, CLOSURE_DETACHED, TYPE_ERROR, UNKNOWN_TYPE, UNKNOWN_VARIABLE,
};
use crate::ir::{CompiledQuery, IrId, IrNode, IrRecordKey, IrStep, IrTree};
use crate::parser::{
    BinaryOp, Expression, FunctionDecl, FunctionParam, LiteralValue, PathStep, PipelineStep, QName,
    SetOp, SurfaceModule, SurfaceNode, TypeExpr, VariableDecl,
};
use crate::resolve::{Arity, BindingId, FunctionKey, ModuleUri, QNameKey};
use crate::types::{stream_item_type, AtomType, NodeKind, SchemaTypeRegistry, Type};

#[derive(Debug, Clone)]
pub struct IrLowerer {
    tree: IrTreeBuilder,
    scopes: Vec<LowerScope>,
    diagnostics: Vec<Diagnostic>,
    aliases: HashMap<String, ModuleUri>,
    source_id: SourceId,
    base_source_map: SourceMapStack,
    next_binding_id: u32,
    binding_types: HashMap<BindingId, Type>,
    lambda_frames: Vec<LambdaFrame>,
    detach_captures: bool,
    schemas: SchemaTypeRegistry,
    policy_bindings: HashMap<BindingId, String>,
    declared_policy_binding_names: Vec<String>,
    declared_policy_functions: Vec<FunctionKey>,
}

impl Default for IrLowerer {
    fn default() -> Self {
        Self {
            tree: IrTreeBuilder::default(),
            scopes: vec![LowerScope::default()],
            diagnostics: Vec::new(),
            aliases: stdlib_aliases(),
            source_id: SourceId(0),
            base_source_map: SourceMapStack::default(),
            next_binding_id: 0,
            binding_types: HashMap::new(),
            lambda_frames: Vec::new(),
            detach_captures: false,
            schemas: SchemaTypeRegistry::default(),
            policy_bindings: HashMap::new(),
            declared_policy_binding_names: Vec::new(),
            declared_policy_functions: Vec::new(),
        }
    }
}

impl IrLowerer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source_id(mut self, source_id: SourceId) -> Self {
        self.source_id = source_id;
        self
    }

    pub fn with_base_source_map(mut self, source_map: SourceMapStack) -> Self {
        self.base_source_map = source_map;
        self
    }

    pub fn with_schema_registry(mut self, schemas: SchemaTypeRegistry) -> Self {
        self.schemas = schemas;
        self
    }

    pub fn with_policy_bindings(mut self, names: impl IntoIterator<Item = String>) -> Self {
        self.declared_policy_binding_names = names.into_iter().collect();
        self
    }

    pub fn with_policy_functions(
        mut self,
        functions: impl IntoIterator<Item = FunctionKey>,
    ) -> Self {
        self.declared_policy_functions = functions.into_iter().collect();
        self
    }

    pub fn detach_captures(mut self, detach: bool) -> Self {
        self.detach_captures = detach;
        self
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn lower_module(mut self, module: &SurfaceModule) -> LowerResult {
        self.predeclare_module_bindings(module);
        for node in &module.nodes {
            if let SurfaceNode::Import(import) = node {
                if let Some(alias) = &import.alias {
                    self.aliases
                        .insert(alias.clone(), ModuleUri(import.uri.clone()));
                }
            }
        }

        let mut root = None;
        for node in &module.nodes {
            match node {
                SurfaceNode::DeclareVariable(var) => {
                    root = Some(self.lower_variable_decl(var));
                }
                SurfaceNode::DeclareFunction(fun) => {
                    root = Some(self.lower_function_decl(fun));
                }
                SurfaceNode::Expression(expr) => {
                    root = Some(self.lower_expr(expr));
                }
                SurfaceNode::Module(_) | SurfaceNode::Import(_) => {}
            }
        }

        let root = root.unwrap_or_else(|| {
            self.push_node(
                IrNode::Sequence(Vec::new()),
                Type::Empty,
                ByteRange::new(0, 0),
                None,
                TransformKind::Query,
            )
        });
        LowerResult {
            query: CompiledQuery {
                tree: self.tree.finish(root),
                policy_bindings: self.policy_bindings,
                source: module.source.clone(),
            },
            diagnostics: self.diagnostics,
        }
    }

    pub fn lower_expression(mut self, expr: &Expression) -> LowerResult {
        let root = self.lower_expression_inner(expr, TransformKind::Query);
        LowerResult {
            query: CompiledQuery {
                tree: self.tree.finish(root),
                policy_bindings: self.policy_bindings,
                source: String::new(),
            },
            diagnostics: self.diagnostics,
        }
    }

    fn predeclare_module_bindings(&mut self, module: &SurfaceModule) {
        let policy_names = self.declared_policy_binding_names.clone();
        for name in policy_names {
            let id = self.allocate_binding_id();
            self.current_scope_mut()
                .variables
                .insert(QNameKey::new(None, name.clone()), id);
            self.binding_types.insert(id, Type::Any);
            self.policy_bindings.insert(id, name);
        }
        let policy_functions = self.declared_policy_functions.clone();
        for key in policy_functions {
            let id = self.allocate_binding_id();
            self.current_scope_mut().functions.insert(key, id);
        }

        for node in &module.nodes {
            match node {
                SurfaceNode::DeclareVariable(var) => {
                    let id = self.allocate_binding_id();
                    self.current_scope_mut()
                        .variables
                        .insert(QNameKey::from_qname(&var.name), id);
                    self.binding_types.insert(id, Type::Any);
                }
                SurfaceNode::DeclareFunction(fun) => {
                    let id = self.allocate_binding_id();
                    self.current_scope_mut().functions.insert(
                        FunctionKey {
                            name: QNameKey::from_qname(&fun.name),
                            arity: Arity(fun.params.len().try_into().unwrap_or(u32::MAX)),
                        },
                        id,
                    );
                }
                _ => {}
            }
        }
    }

    fn lower_variable_decl(&mut self, var: &VariableDecl) -> IrId {
        let value = self.lower_expr(&var.value);
        let ty = self.type_of(value).cloned().unwrap_or(Type::Any);
        let id = self.lookup_variable_id(&var.name).unwrap_or_else(|| {
            self.declare_variable_with_type(QNameKey::from_qname(&var.name), ty.clone())
        });
        self.binding_types.insert(id, ty.clone());
        let node = IrNode::Let {
            name: id,
            value,
            body: value,
        };
        self.push_node(node, ty, var.range, Some(id), TransformKind::Query)
    }

    fn lower_function_decl(&mut self, fun: &FunctionDecl) -> IrId {
        self.push_scope();
        let params: Vec<(BindingId, Type)> = fun
            .params
            .iter()
            .map(|param| {
                let ty = self.type_expr(&param.type_annotation);
                let binding =
                    self.declare_variable_with_type(QNameKey::from_qname(&param.name), ty.clone());
                (binding, ty)
            })
            .collect();
        let body = self.lower_expr(&fun.body);
        let ret = self.type_of(body).cloned().unwrap_or(Type::Any);
        self.pop_scope();

        let captures = Vec::new();
        let id = self
            .lookup_function_id(&fun.name, fun.params.len())
            .unwrap_or_else(|| {
                self.declare_function(QNameKey::from_qname(&fun.name), fun.params.len())
            });
        self.push_node(
            IrNode::Lambda {
                params: params.clone(),
                body,
                captures,
            },
            Type::Lambda {
                params: params.into_iter().map(|(_, ty)| ty).collect(),
                ret: Box::new(ret),
            },
            fun.range,
            Some(id),
            TransformKind::Query,
        )
    }

    fn lower_expr(&mut self, expr: &Expression) -> IrId {
        self.lower_expression_inner(expr, TransformKind::Query)
    }

    fn lower_expression_inner(&mut self, expr: &Expression, transform: TransformKind) -> IrId {
        match expr {
            Expression::Literal(value, range) => self.lower_literal(value, *range, transform),
            Expression::Name(name, range) => {
                let binding = self.lookup_variable_id(name).or_else(|| {
                    self.emit(
                        UNKNOWN_VARIABLE,
                        format!(
                            "unknown variable `{}`",
                            QNameKey::from_qname(name).display()
                        ),
                        *range,
                        Severity::Error,
                    );
                    None
                });
                let ty = binding
                    .and_then(|binding| self.binding_types.get(&binding).cloned())
                    .unwrap_or(Type::Any);
                self.push_node(
                    IrNode::LocalVar(binding.unwrap_or(BindingId(u32::MAX))),
                    ty,
                    *range,
                    binding,
                    transform,
                )
            }
            Expression::LeadingDot(range) => {
                self.push_node(IrNode::LeadingDot, Type::Any, *range, None, transform)
            }
            Expression::Path { steps, range } => self.lower_path(steps, *range, transform),
            Expression::Pipeline {
                source,
                steps,
                range,
            } => {
                let source = self.lower_expr(source);
                let steps = steps.iter().map(|step| self.lower_step(step)).collect();
                self.push_node(
                    IrNode::Pipeline { source, steps },
                    Type::Any,
                    *range,
                    None,
                    transform,
                )
            }
            Expression::BinaryOp {
                op,
                lhs,
                rhs,
                range,
            } if *op == BinaryOp::Is => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                self.push_node(
                    IrNode::Is { lhs, rhs },
                    Type::atom(AtomType::Boolean),
                    *range,
                    None,
                    transform,
                )
            }
            Expression::BinaryOp {
                op,
                lhs,
                rhs,
                range,
            } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                if *op == BinaryOp::Minus {
                    return self.lower_minus(lhs, rhs, *range, transform);
                }
                if numeric_op(*op) {
                    return self.lower_numeric_binary(*op, lhs, rhs, *range, transform);
                }
                let ty = if comparison_op(*op) || matches!(op, BinaryOp::And | BinaryOp::Or) {
                    Type::atom(AtomType::Boolean)
                } else {
                    Type::Any
                };
                self.push_node(
                    IrNode::BinaryOp { op: *op, lhs, rhs },
                    ty,
                    *range,
                    None,
                    transform,
                )
            }
            Expression::UnaryOp { op, operand, range } => {
                let operand = self.lower_expr(operand);
                let ty = if matches!(op, crate::parser::UnaryOp::Not) {
                    Type::atom(AtomType::Boolean)
                } else {
                    self.type_of(operand).cloned().unwrap_or(Type::Any)
                };
                self.push_node(
                    IrNode::UnaryOp { op: *op, operand },
                    ty,
                    *range,
                    None,
                    transform,
                )
            }
            Expression::SetOp {
                op,
                lhs,
                rhs,
                range,
            } => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                self.push_node(
                    IrNode::SetOp { op: *op, lhs, rhs },
                    Type::stream(Type::Any),
                    *range,
                    None,
                    transform,
                )
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                range,
            } => {
                let cond = self.lower_expr(cond);
                let then_branch = self.lower_expr(then_branch);
                let else_branch = self.lower_expr(else_branch);
                let ty = self.type_of(then_branch).cloned().unwrap_or(Type::Any);
                self.push_node(
                    IrNode::If {
                        cond,
                        then_branch,
                        else_branch,
                    },
                    ty,
                    *range,
                    None,
                    transform,
                )
            }
            Expression::Let {
                name,
                value,
                body,
                range,
            } => {
                let value = self.lower_expr(value);
                let value_ty = self.type_of(value).cloned().unwrap_or(Type::Any);
                self.push_scope();
                let binding = self.declare_variable_with_type(QNameKey::from_qname(name), value_ty);
                let body = self.lower_expr(body);
                let ty = self.type_of(body).cloned().unwrap_or(Type::Any);
                self.pop_scope();
                self.push_node(
                    IrNode::Let {
                        name: binding,
                        value,
                        body,
                    },
                    ty,
                    *range,
                    Some(binding),
                    transform,
                )
            }
            Expression::For {
                var,
                source,
                body,
                range,
            } => {
                let source = self.lower_expr(source);
                let source_ty = self.type_of(source).cloned().unwrap_or(Type::Any);
                let item_ty = stream_item_type(&source_ty).unwrap_or(source_ty);
                self.push_scope();
                let binding = self.declare_variable_with_type(QNameKey::from_qname(var), item_ty);
                let body = self.lower_expr(body);
                let body_ty = self.type_of(body).cloned().unwrap_or(Type::Any);
                self.pop_scope();
                self.push_node(
                    IrNode::For {
                        var: binding,
                        source,
                        body,
                    },
                    Type::stream(body_ty),
                    *range,
                    Some(binding),
                    transform,
                )
            }
            Expression::Quantified {
                kind,
                var,
                source,
                predicate,
                range,
            } => {
                let source = self.lower_expr(source);
                let source_ty = self.type_of(source).cloned().unwrap_or(Type::Any);
                let item_ty = stream_item_type(&source_ty).unwrap_or(source_ty);
                self.push_scope();
                let binding = self.declare_variable_with_type(QNameKey::from_qname(var), item_ty);
                let predicate = self.lower_expr(predicate);
                self.pop_scope();
                self.push_node(
                    IrNode::Quantified {
                        kind: *kind,
                        var: binding,
                        source,
                        predicate,
                    },
                    Type::atom(AtomType::Boolean),
                    *range,
                    Some(binding),
                    transform,
                )
            }
            Expression::Record { entries, range } => {
                let entries = entries
                    .iter()
                    .map(|entry| {
                        let key = match &entry.key {
                            crate::parser::RecordKey::Static { value, .. } => {
                                IrRecordKey::Static(value.clone())
                            }
                            crate::parser::RecordKey::Computed { expr, .. } => {
                                IrRecordKey::Computed(self.lower_expr(expr))
                            }
                        };
                        (key, self.lower_expr(&entry.value))
                    })
                    .collect();
                self.push_node(IrNode::Record(entries), Type::Any, *range, None, transform)
            }
            Expression::Sequence { items, range } => {
                let items: Vec<IrId> = items.iter().map(|item| self.lower_expr(item)).collect();
                let ty = self.sequence_type(&items);
                self.push_node(IrNode::Sequence(items), ty, *range, None, transform)
            }
            Expression::Call {
                callee,
                args,
                range,
            } => self.lower_call(callee, args, *range, transform),
            Expression::Lambda {
                params,
                body,
                range,
            } => self.lower_lambda(params, body, *range, transform),
            Expression::InstanceOf { value, ty, range } => {
                let value = self.lower_expr(value);
                let ty = self.type_expr(&Some(ty.clone()));
                self.push_node(
                    IrNode::InstanceOf { value, ty },
                    Type::atom(AtomType::Boolean),
                    *range,
                    None,
                    transform,
                )
            }
            Expression::CastAs { value, ty, range } => {
                let value = self.lower_expr(value);
                let ty = self.type_expr(&Some(ty.clone()));
                self.push_node(
                    IrNode::CastAs {
                        value,
                        ty: ty.clone(),
                    },
                    ty,
                    *range,
                    None,
                    transform,
                )
            }
            Expression::TreatAs { value, ty, range } => {
                let value = self.lower_expr(value);
                let ty = self.type_expr(&Some(ty.clone()));
                self.push_node(
                    IrNode::TreatAs {
                        value,
                        ty: ty.clone(),
                    },
                    ty,
                    *range,
                    None,
                    transform,
                )
            }
        }
    }

    fn lower_literal(
        &mut self,
        value: &LiteralValue,
        range: ByteRange,
        transform: TransformKind,
    ) -> IrId {
        let (node, ty) = match value {
            LiteralValue::String(value) => (
                IrNode::LitString(value.clone()),
                Type::atom(AtomType::String),
            ),
            LiteralValue::Integer(value) => (IrNode::LitInt(*value), Type::atom(AtomType::Integer)),
            LiteralValue::Decimal(value) => (
                IrNode::LitDecimal(value.clone()),
                Type::atom(AtomType::Decimal),
            ),
            LiteralValue::Double(value) => {
                (IrNode::LitDouble(*value), Type::atom(AtomType::Double))
            }
            LiteralValue::Boolean(value) => {
                (IrNode::LitBool(*value), Type::atom(AtomType::Boolean))
            }
            LiteralValue::Null => (IrNode::LitNull, Type::Empty),
        };
        self.push_node(node, ty, range, None, transform)
    }

    fn lower_path(
        &mut self,
        steps: &[PathStep],
        range: ByteRange,
        transform: TransformKind,
    ) -> IrId {
        let mut lowered = steps.iter().map(|step| self.lower_path_step(step));
        let Some(first) = lowered.next() else {
            return self.push_node(
                IrNode::Sequence(Vec::new()),
                Type::Empty,
                range,
                None,
                transform,
            );
        };
        let rest = lowered.map(IrStep::Lambda).collect();
        self.push_node(
            IrNode::Pipeline {
                source: first,
                steps: rest,
            },
            Type::stream(Type::Node(NodeKind::Node)),
            range,
            None,
            transform,
        )
    }

    fn lower_path_step(&mut self, step: &PathStep) -> IrId {
        match step {
            PathStep::Axis {
                axis,
                name_test,
                predicates,
                range,
            } => {
                let predicates = predicates
                    .iter()
                    .map(|predicate| self.lower_expr(predicate))
                    .collect();
                self.push_node(
                    IrNode::AxisStep {
                        axis: *axis,
                        name_test: name_test.clone(),
                        predicates,
                    },
                    Type::stream(Type::Node(NodeKind::Node)),
                    *range,
                    None,
                    TransformKind::Query,
                )
            }
            PathStep::Parent(range) => self.push_node(
                IrNode::Parent,
                Type::stream(Type::Node(NodeKind::Node)),
                *range,
                None,
                TransformKind::Query,
            ),
            PathStep::Self_(range) => self.push_node(
                IrNode::Self_,
                Type::stream(Type::Node(NodeKind::Node)),
                *range,
                None,
                TransformKind::Query,
            ),
        }
    }

    fn lower_step(&mut self, step: &PipelineStep) -> IrStep {
        match step {
            PipelineStep::Named { name, args, .. } => {
                let args = args.iter().map(|arg| self.lower_expr(arg)).collect();
                if let Some(module) = self.stdlib_module_for(name) {
                    IrStep::NamedStdlib {
                        module,
                        name: name.clone(),
                        args,
                    }
                } else {
                    IrStep::Named {
                        binding: self.lookup_function_id(name, args_len(&args)),
                        name: name.clone(),
                        args,
                    }
                }
            }
            PipelineStep::Lambda { lambda, .. } => IrStep::Lambda(self.lower_lambda(
                &[],
                lambda,
                lambda.range(),
                TransformKind::QueryStep,
            )),
        }
    }

    fn lower_lambda(
        &mut self,
        params: &[FunctionParam],
        body_expr: &Expression,
        range: ByteRange,
        transform: TransformKind,
    ) -> IrId {
        self.push_scope();
        let frame_index = self.lambda_frames.len();
        self.lambda_frames.push(LambdaFrame {
            local_scope_depth: self.scopes.len(),
            captures: BTreeSet::new(),
        });
        let param_bindings: Vec<(BindingId, Type)> = params
            .iter()
            .map(|param| {
                let ty = self.type_expr(&param.type_annotation);
                let binding =
                    self.declare_variable_with_type(QNameKey::from_qname(&param.name), ty.clone());
                (binding, ty)
            })
            .collect();
        let body = self.lower_expression_inner(body_expr, transform.clone());
        let frame = self.lambda_frames.remove(frame_index);
        self.pop_scope();

        let captures: Vec<BindingId> = frame.captures.into_iter().collect();
        if self.detach_captures && !captures.is_empty() {
            self.emit(
                CLOSURE_DETACHED,
                "closure capture detached from host scope",
                range,
                Severity::Info,
            );
        }
        let ret = self.type_of(body).cloned().unwrap_or(Type::Any);
        self.push_node(
            IrNode::Lambda {
                params: param_bindings.clone(),
                body,
                captures,
            },
            Type::Lambda {
                params: param_bindings.into_iter().map(|(_, ty)| ty).collect(),
                ret: Box::new(ret),
            },
            range,
            None,
            transform,
        )
    }

    fn lower_call(
        &mut self,
        callee: &Expression,
        args: &[Expression],
        range: ByteRange,
        transform: TransformKind,
    ) -> IrId {
        let arg_ids: Vec<IrId> = args.iter().map(|arg| self.lower_expr(arg)).collect();
        if let Expression::Name(name, _) = callee {
            if let Some(module) = self.stdlib_module_for(name) {
                return self.push_node(
                    IrNode::StdlibCall {
                        module,
                        name: name.clone(),
                        args: arg_ids,
                    },
                    Type::Any,
                    range,
                    None,
                    transform,
                );
            }
            if let Some(binding) = self.lookup_function_id(name, arg_ids.len()) {
                let callee = self.push_node(
                    IrNode::FunctionRef(binding),
                    Type::Any,
                    name.range,
                    Some(binding),
                    TransformKind::Query,
                );
                return self.push_node(
                    IrNode::Call {
                        callee,
                        args: arg_ids,
                    },
                    Type::Any,
                    range,
                    Some(binding),
                    transform,
                );
            }
        }

        let callee = self.lower_expr(callee);
        self.push_node(
            IrNode::Call {
                callee,
                args: arg_ids,
            },
            Type::Any,
            range,
            None,
            transform,
        )
    }

    fn lower_minus(
        &mut self,
        lhs: IrId,
        rhs: IrId,
        range: ByteRange,
        transform: TransformKind,
    ) -> IrId {
        let lhs_ty = self.type_of(lhs).cloned().unwrap_or(Type::Any);
        let rhs_ty = self.type_of(rhs).cloned().unwrap_or(Type::Any);
        match (minus_operand_shape(&lhs_ty), minus_operand_shape(&rhs_ty)) {
            (MinusOperandShape::Numeric, MinusOperandShape::Numeric) => {
                let ty = if lhs_ty == rhs_ty {
                    lhs_ty.clone()
                } else {
                    self.emit(
                        TYPE_ERROR,
                        format!(
                            "operator `-` requires matching numeric operand types, got `{lhs_ty:?}` and `{rhs_ty:?}`; use an explicit num:* conversion"
                        ),
                        range,
                        Severity::Error,
                    );
                    Type::Any
                };
                self.push_node(
                    IrNode::BinaryOp {
                        op: BinaryOp::Minus,
                        lhs,
                        rhs,
                    },
                    ty,
                    range,
                    None,
                    transform,
                )
            }
            (MinusOperandShape::Stream, MinusOperandShape::Stream) => self.push_node(
                IrNode::SetOp {
                    op: SetOp::Difference,
                    lhs,
                    rhs,
                },
                Type::stream(common_stream_item_type(&lhs_ty, &rhs_ty).unwrap_or(Type::Any)),
                range,
                None,
                transform,
            ),
            (MinusOperandShape::Unknown, _) | (_, MinusOperandShape::Unknown) => self.push_node(
                IrNode::BinaryOp {
                    op: BinaryOp::Minus,
                    lhs,
                    rhs,
                },
                Type::Any,
                range,
                None,
                transform,
            ),
            (MinusOperandShape::Numeric, MinusOperandShape::Stream)
            | (MinusOperandShape::Stream, MinusOperandShape::Numeric) => {
                self.emit(
                    TYPE_ERROR,
                    format!(
                        "operator `-` cannot mix numeric and stream operands: `{lhs_ty:?}` and `{rhs_ty:?}`"
                    ),
                    range,
                    Severity::Error,
                );
                self.push_node(
                    IrNode::BinaryOp {
                        op: BinaryOp::Minus,
                        lhs,
                        rhs,
                    },
                    Type::Any,
                    range,
                    None,
                    transform,
                )
            }
            (MinusOperandShape::Other, _) | (_, MinusOperandShape::Other) => {
                self.emit(
                    TYPE_ERROR,
                    format!(
                        "operator `-` requires numeric or stream operands, got `{lhs_ty:?}` and `{rhs_ty:?}`"
                    ),
                    range,
                    Severity::Error,
                );
                self.push_node(
                    IrNode::BinaryOp {
                        op: BinaryOp::Minus,
                        lhs,
                        rhs,
                    },
                    Type::Any,
                    range,
                    None,
                    transform,
                )
            }
        }
    }

    fn lower_numeric_binary(
        &mut self,
        op: BinaryOp,
        lhs: IrId,
        rhs: IrId,
        range: ByteRange,
        transform: TransformKind,
    ) -> IrId {
        let lhs_ty = self.type_of(lhs).cloned().unwrap_or(Type::Any);
        let rhs_ty = self.type_of(rhs).cloned().unwrap_or(Type::Any);
        let ty = if lhs_ty.is_any() || rhs_ty.is_any() {
            Type::Any
        } else if lhs_ty.is_numeric_atom() && rhs_ty.is_numeric_atom() {
            if lhs_ty == rhs_ty {
                lhs_ty.clone()
            } else {
                self.emit(
                    TYPE_ERROR,
                    format!(
                        "operator `{}` requires matching numeric operand types, got `{lhs_ty:?}` and `{rhs_ty:?}`; use an explicit num:* conversion",
                        op.symbol()
                    ),
                    range,
                    Severity::Error,
                );
                Type::Any
            }
        } else {
            self.emit(
                TYPE_ERROR,
                format!(
                    "operator `{}` requires numeric operands, got `{lhs_ty:?}` and `{rhs_ty:?}`",
                    op.symbol()
                ),
                range,
                Severity::Error,
            );
            Type::Any
        };
        self.push_node(
            IrNode::BinaryOp { op, lhs, rhs },
            ty,
            range,
            None,
            transform,
        )
    }

    fn stdlib_module_for(&self, name: &QName) -> Option<ModuleUri> {
        if name.prefix.is_none() && name.local == "read" {
            return Some(ModuleUri("cem:stdlib/content-types".to_owned()));
        }
        if name.prefix.is_none() && matches!(name.local.as_str(), "any" | "all") {
            return Some(ModuleUri("cem:stdlib/sequence".to_owned()));
        }
        name.prefix
            .as_ref()
            .and_then(|prefix| self.aliases.get(prefix).cloned())
            .filter(|module| module.0.starts_with("cem:stdlib/"))
    }

    fn lookup_variable_id(&mut self, name: &QName) -> Option<BindingId> {
        let key = QNameKey::from_qname(name);
        for (index, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(binding) = scope.variables.get(&key).copied() {
                if let Some(frame) = self.lambda_frames.last_mut() {
                    if index + 1 < frame.local_scope_depth {
                        frame.captures.insert(binding);
                    }
                }
                return Some(binding);
            }
        }
        None
    }

    fn lookup_function_id(&self, name: &QName, arity: usize) -> Option<BindingId> {
        let key = FunctionKey {
            name: QNameKey::from_qname(name),
            arity: Arity(arity.try_into().unwrap_or(u32::MAX)),
        };
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.functions.get(&key).copied())
    }

    fn declare_variable_with_type(&mut self, name: QNameKey, ty: Type) -> BindingId {
        let id = self.allocate_binding_id();
        self.current_scope_mut().variables.insert(name, id);
        self.binding_types.insert(id, ty);
        id
    }

    fn declare_function(&mut self, name: QNameKey, arity: usize) -> BindingId {
        let id = self.allocate_binding_id();
        self.current_scope_mut().functions.insert(
            FunctionKey {
                name,
                arity: Arity(arity.try_into().unwrap_or(u32::MAX)),
            },
            id,
        );
        id
    }

    fn allocate_binding_id(&mut self) -> BindingId {
        let id = BindingId(self.next_binding_id);
        self.next_binding_id = self.next_binding_id.saturating_add(1);
        id
    }

    fn push_scope(&mut self) {
        self.scopes.push(LowerScope::default());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn current_scope_mut(&mut self) -> &mut LowerScope {
        self.scopes.last_mut().expect("lowerer always has a scope")
    }

    fn push_node(
        &mut self,
        node: IrNode,
        ty: Type,
        range: ByteRange,
        resolution: Option<BindingId>,
        transform: TransformKind,
    ) -> IrId {
        self.tree
            .push(node, ty, self.source_map(range, transform), resolution)
    }

    fn source_map(&self, range: ByteRange, transform: TransformKind) -> SourceMapStack {
        let mut stack = self.base_source_map.clone();
        stack.push(SourceMapFrame {
            source_id: self.source_id,
            span: FrameSpan::Single(range),
            transform,
        });
        stack
    }

    fn type_of(&self, id: IrId) -> Option<&Type> {
        self.tree.types.get(id.0 as usize)
    }

    fn sequence_type(&self, items: &[IrId]) -> Type {
        if items.is_empty() {
            return Type::Empty;
        }
        let mut item_ty = Type::Empty;
        for item in items {
            let next_ty = self.type_of(*item).cloned().unwrap_or(Type::Any);
            item_ty = common_lowered_type(&item_ty, &next_ty);
        }
        Type::stream(item_ty)
    }

    fn type_expr(&mut self, ty: &Option<TypeExpr>) -> Type {
        let Some(ty) = ty else {
            return Type::Any;
        };
        let key = QNameKey::from_qname(&ty.name);
        if let Some(ty) = builtin_type(&key) {
            return ty;
        }
        if let Some(schema_id) = self.schemas.resolve(&key) {
            return Type::SchemaElement(schema_id);
        }
        self.emit(
            UNKNOWN_TYPE,
            format!("unknown type `{}`", key.display()),
            ty.range,
            Severity::Error,
        );
        Type::Any
    }

    fn emit(
        &mut self,
        code: DiagnosticCode,
        message: impl Into<String>,
        range: ByteRange,
        severity: Severity,
    ) {
        self.diagnostics.push(Diagnostic {
            uri: None,
            line: None,
            column: None,
            byte_offset: Some(range.start),
            code: code.into(),
            severity,
            message: message.into(),
            node: None,
            details: None,
            source_map: Some(self.source_map(range, TransformKind::Query)),
        });
    }
}

#[derive(Debug, Clone)]
pub struct LowerResult {
    pub query: CompiledQuery,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default)]
struct IrTreeBuilder {
    nodes: Vec<IrNode>,
    source_maps: Vec<SourceMapStack>,
    types: Vec<Type>,
    resolutions: Vec<Option<BindingId>>,
}

impl IrTreeBuilder {
    fn push(
        &mut self,
        node: IrNode,
        ty: Type,
        source_map: SourceMapStack,
        resolution: Option<BindingId>,
    ) -> IrId {
        let id = IrId(self.nodes.len().try_into().unwrap_or(u32::MAX));
        self.nodes.push(node);
        self.types.push(ty);
        self.source_maps.push(source_map);
        self.resolutions.push(resolution);
        id
    }

    fn finish(self, root: IrId) -> IrTree {
        IrTree {
            nodes: self.nodes,
            root,
            source_maps: self.source_maps,
            types: self.types,
            resolutions: self.resolutions,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct LowerScope {
    variables: HashMap<QNameKey, BindingId>,
    functions: HashMap<FunctionKey, BindingId>,
}

#[derive(Debug, Clone)]
struct LambdaFrame {
    local_scope_depth: usize,
    captures: BTreeSet<BindingId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MinusOperandShape {
    Numeric,
    Stream,
    Unknown,
    Other,
}

fn minus_operand_shape(ty: &Type) -> MinusOperandShape {
    if ty.is_any() {
        MinusOperandShape::Unknown
    } else if ty.is_numeric_atom() {
        MinusOperandShape::Numeric
    } else if ty.is_stream_like() {
        MinusOperandShape::Stream
    } else {
        MinusOperandShape::Other
    }
}

fn common_stream_item_type(lhs: &Type, rhs: &Type) -> Option<Type> {
    match (stream_item_type(lhs), stream_item_type(rhs)) {
        (Some(left), Some(right)) if left == right => Some(left),
        (Some(Type::Empty), Some(item)) | (Some(item), Some(Type::Empty)) => Some(item),
        (Some(_), Some(_)) => Some(Type::Any),
        _ => None,
    }
}

fn common_lowered_type(lhs: &Type, rhs: &Type) -> Type {
    if matches!(lhs, Type::Empty) {
        rhs.clone()
    } else if matches!(rhs, Type::Empty) || lhs == rhs {
        lhs.clone()
    } else {
        Type::Any
    }
}

fn args_len(args: &[IrId]) -> usize {
    args.len()
}

fn comparison_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::EqEq
            | BinaryOp::BangEq
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Is
    )
}

fn numeric_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Plus | BinaryOp::Star | BinaryOp::Slash | BinaryOp::Percent
    )
}

fn builtin_type(key: &QNameKey) -> Option<Type> {
    let unprefixed = key.prefix.is_none();
    let xs = key.prefix.as_deref() == Some("xs");
    match (key.local.as_str(), unprefixed || xs) {
        ("string", true) => Some(Type::atom(AtomType::String)),
        ("integer", true) => Some(Type::atom(AtomType::Integer)),
        ("decimal", true) => Some(Type::atom(AtomType::Decimal)),
        ("double", true) => Some(Type::atom(AtomType::Double)),
        ("boolean", true) => Some(Type::atom(AtomType::Boolean)),
        ("date", true) => Some(Type::atom(AtomType::Date)),
        ("dateTime", true) => Some(Type::atom(AtomType::DateTime)),
        ("duration", true) => Some(Type::atom(AtomType::Duration)),
        ("anyURI", true) => Some(Type::atom(AtomType::AnyUri)),
        ("node", true) => Some(Type::Node(NodeKind::Node)),
        ("text", true) => Some(Type::Node(NodeKind::Text)),
        ("comment", true) => Some(Type::Node(NodeKind::Comment)),
        ("processing-instruction", true) => Some(Type::Node(NodeKind::ProcessingInstruction)),
        ("document-node", true) => Some(Type::Node(NodeKind::DocumentNode)),
        _ => None,
    }
}

fn stdlib_aliases() -> HashMap<String, ModuleUri> {
    [
        ("seq", "cem:stdlib/sequence"),
        ("str", "cem:stdlib/strings"),
        ("num", "cem:stdlib/numbers"),
        ("dt", "cem:stdlib/datetime"),
        ("dom", "cem:stdlib/dom"),
        ("report", "cem:stdlib/report"),
        ("state", "cem:stdlib/state"),
        ("tpl", "cem:stdlib/template"),
        ("cemml", "cem:stdlib/cemml"),
        ("ct", "cem:stdlib/content-types"),
        ("user", "cem:stdlib/user"),
    ]
    .into_iter()
    .map(|(prefix, uri)| (prefix.to_owned(), ModuleUri(uri.to_owned())))
    .collect()
}
