//! Layer 4: static type checker.

use std::collections::{BTreeSet, HashMap};

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::source::ByteRange;
use serde_json::json;

use crate::diagnostics::{
    self as ql_diagnostics, DiagnosticCode, CROSS_TYPE_COMPARE, TYPE_ERROR, UNKNOWN_FUNCTION,
    UNKNOWN_TYPE, UNKNOWN_VARIABLE,
};
use crate::parser::{
    BinaryOp, Expression, FunctionDecl, FunctionParam, ImportDecl, LiteralValue, PathStep,
    PipelineStep, QName, SurfaceModule, SurfaceNode, TypeExpr, UnaryOp, VariableDecl,
};
use crate::resolve::{Arity, QNameKey, SchemaTypeId};

pub mod lattice;
pub mod subtype;

pub use lattice::TypeLattice;
pub use subtype::SubtypeChecker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Node(NodeKind),
    SchemaElement(SchemaTypeId),
    Atom(AtomType),
    Record(Vec<RecordField>),
    Array(Box<Type>),
    Stream(Box<Type>),
    Lambda {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Resource {
        content_type: ContentType,
        schema: Option<SchemaTypeId>,
    },
    Any,
    Empty,
}

impl Type {
    pub fn is_subtype_of(&self, expected: &Type, schemas: &SchemaTypeRegistry) -> bool {
        TypeLattice::new(schemas).is_subtype(self, expected)
    }

    pub fn atom(atom: AtomType) -> Self {
        Self::Atom(atom)
    }

    pub fn stream(item: Type) -> Self {
        Self::Stream(Box::new(item))
    }

    pub(crate) fn is_any(&self) -> bool {
        matches!(self, Type::Any)
    }

    pub(crate) fn is_numeric_atom(&self) -> bool {
        matches!(
            self,
            Type::Atom(AtomType::Integer | AtomType::Decimal | AtomType::Double)
        )
    }

    pub(crate) fn is_stream_like(&self) -> bool {
        matches!(self, Type::Stream(_) | Type::Empty)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomType {
    String,
    Integer,
    Decimal,
    Double,
    Boolean,
    Date,
    DateTime,
    Duration,
    AnyUri,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Node,
    Element(QNameKey),
    Attribute(QNameKey),
    Text,
    Comment,
    ProcessingInstruction,
    DocumentNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentType(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaTypeInfo {
    pub id: SchemaTypeId,
    pub name: QNameKey,
    pub element_name: QNameKey,
    pub structural_supertypes: Vec<SchemaTypeId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaTypeRegistry {
    by_id: HashMap<SchemaTypeId, SchemaTypeInfo>,
    by_name: HashMap<QNameKey, SchemaTypeId>,
}

impl SchemaTypeRegistry {
    pub fn insert(&mut self, info: SchemaTypeInfo) -> Option<SchemaTypeInfo> {
        self.by_name.insert(info.name.clone(), info.id);
        self.by_id.insert(info.id, info)
    }

    pub fn get(&self, id: SchemaTypeId) -> Option<&SchemaTypeInfo> {
        self.by_id.get(&id)
    }

    pub fn resolve(&self, name: &QNameKey) -> Option<SchemaTypeId> {
        self.by_name.get(name).copied()
    }

    pub fn is_structural_subtype(&self, actual: SchemaTypeId, expected: SchemaTypeId) -> bool {
        if actual == expected {
            return true;
        }
        let Some(info) = self.get(actual) else {
            return false;
        };
        info.structural_supertypes
            .iter()
            .any(|parent| self.is_structural_subtype(*parent, expected))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignatureKey {
    pub name: QNameKey,
    pub arity: Arity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub name: QNameKey,
    pub params: Vec<Type>,
    pub ret: Type,
}

impl FunctionSignature {
    pub fn key(&self) -> FunctionSignatureKey {
        FunctionSignatureKey {
            name: self.name.clone(),
            arity: Arity(self.params.len().try_into().unwrap_or(u32::MAX)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyConfig {
    type_error_severity: Severity,
    resolution_error_severity: Severity,
    emit_cross_type_compare: bool,
}

impl Default for TyConfig {
    fn default() -> Self {
        Self {
            type_error_severity: Severity::Error,
            resolution_error_severity: Severity::Error,
            emit_cross_type_compare: true,
        }
    }
}

impl TyConfig {
    pub fn strict() -> Self {
        Self::default()
    }

    pub fn dev_profile() -> Self {
        Self {
            type_error_severity: Severity::Warning,
            resolution_error_severity: Severity::Warning,
            emit_cross_type_compare: false,
        }
    }

    pub fn cache_stamp(&self) -> String {
        format!(
            "ty-config/1;type-error={:?};resolution={:?};cross-type-compare={}",
            self.type_error_severity, self.resolution_error_severity, self.emit_cross_type_compare
        )
    }

    fn severity_for(&self, code: DiagnosticCode) -> Option<Severity> {
        if code == TYPE_ERROR {
            Some(self.type_error_severity)
        } else if matches!(code, UNKNOWN_TYPE | UNKNOWN_FUNCTION | UNKNOWN_VARIABLE) {
            Some(self.resolution_error_severity)
        } else if code == CROSS_TYPE_COMPARE && self.emit_cross_type_compare {
            Some(Severity::Warning)
        } else if code == CROSS_TYPE_COMPARE {
            None
        } else {
            Some(Severity::Error)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TypeReport {
    pub diagnostics: Vec<Diagnostic>,
    pub root_type: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct TypeChecker {
    pub config: TyConfig,
    pub schemas: SchemaTypeRegistry,
    pub functions: HashMap<FunctionSignatureKey, FunctionSignature>,
    imported_prefixes: BTreeSet<String>,
    scopes: Vec<HashMap<QNameKey, Type>>,
    diagnostics: Vec<Diagnostic>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self {
            config: TyConfig::default(),
            schemas: SchemaTypeRegistry::default(),
            functions: HashMap::new(),
            imported_prefixes: BTreeSet::new(),
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
        }
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: TyConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn register_schema_type(&mut self, info: SchemaTypeInfo) {
        self.schemas.insert(info);
    }

    pub fn register_function(&mut self, signature: FunctionSignature) {
        self.functions.insert(signature.key(), signature);
    }

    pub fn seed_runtime_import_surface(&mut self, module: &SurfaceModule) {
        self.register_default_stdlib_aliases();
        self.register_bare_stdlib_helpers();
        for node in &module.nodes {
            if let SurfaceNode::Import(import) = node {
                self.register_import_surface(import);
            }
        }
    }

    pub fn declare_variable(&mut self, name: QNameKey, ty: Type) {
        self.current_scope_mut().insert(name, ty);
    }

    pub fn infer(&mut self, expr: &Expression) -> Type {
        self.infer_expression(expr)
    }

    pub fn check(&mut self, expr: &Expression, expected: &Type) -> bool {
        let actual = self.infer_expression(expr);
        self.expect_subtype(&actual, expected, expr.range())
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn check_surface_module(&mut self, module: &SurfaceModule) -> TypeReport {
        let diagnostics_start = self.diagnostics.len();
        let mut root_type = None;
        self.push_scope();
        self.declare_module_signatures(module);
        for node in &module.nodes {
            match node {
                SurfaceNode::DeclareVariable(var) => {
                    root_type = Some(self.check_variable_decl(var));
                }
                SurfaceNode::DeclareFunction(fun) => {
                    root_type = Some(self.check_function_decl(fun));
                }
                SurfaceNode::Expression(expr) => {
                    root_type = Some(self.infer_expression(expr));
                }
                SurfaceNode::Module(_) | SurfaceNode::Import(_) => {}
            }
        }
        self.pop_scope();
        TypeReport {
            diagnostics: self.diagnostics[diagnostics_start..].to_vec(),
            root_type,
        }
    }

    fn declare_module_signatures(&mut self, module: &SurfaceModule) {
        for node in &module.nodes {
            match node {
                SurfaceNode::DeclareVariable(var) => {
                    self.declare_variable(QNameKey::from_qname(&var.name), Type::Any);
                }
                SurfaceNode::DeclareFunction(fun) => {
                    let params = fun
                        .params
                        .iter()
                        .map(|param| self.param_type(param))
                        .collect();
                    self.register_function(FunctionSignature {
                        name: QNameKey::from_qname(&fun.name),
                        params,
                        ret: Type::Any,
                    });
                }
                _ => {}
            }
        }
    }

    fn check_variable_decl(&mut self, var: &VariableDecl) -> Type {
        let ty = self.infer_expression(&var.value);
        self.declare_variable(QNameKey::from_qname(&var.name), ty.clone());
        ty
    }

    fn check_function_decl(&mut self, fun: &FunctionDecl) -> Type {
        self.push_scope();
        for param in &fun.params {
            let param_ty = self.param_type(param);
            self.declare_variable(QNameKey::from_qname(&param.name), param_ty);
        }
        let ret = self.infer_expression(&fun.body);
        self.pop_scope();

        let params = fun
            .params
            .iter()
            .map(|param| self.param_type(param))
            .collect();
        self.register_function(FunctionSignature {
            name: QNameKey::from_qname(&fun.name),
            params,
            ret: ret.clone(),
        });
        Type::Lambda {
            params: fun
                .params
                .iter()
                .map(|param| self.param_type(param))
                .collect(),
            ret: Box::new(ret),
        }
    }

    fn param_type(&mut self, param: &FunctionParam) -> Type {
        param
            .type_annotation
            .as_ref()
            .map(|ty| self.resolve_type_expr(ty))
            .unwrap_or(Type::Any)
    }

    fn infer_expression(&mut self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(value, _) => self.infer_literal(value),
            Expression::Name(name, range) => self.lookup_variable(name).unwrap_or_else(|| {
                if self.is_imported_name(name) {
                    return Type::Any;
                }
                self.emit(
                    UNKNOWN_VARIABLE,
                    format!(
                        "unknown variable `{}`",
                        QNameKey::from_qname(name).display()
                    ),
                    *range,
                );
                Type::Any
            }),
            Expression::LeadingDot(_) => Type::Any,
            Expression::Path { steps, .. } => self.infer_path(steps),
            Expression::Pipeline { source, steps, .. } => self.infer_pipeline(source, steps),
            Expression::BinaryOp {
                op,
                lhs,
                rhs,
                range,
            } => self.infer_binary(*op, lhs, rhs, *range),
            Expression::UnaryOp { op, operand, range } => self.infer_unary(*op, operand, *range),
            Expression::SetOp {
                lhs, rhs, range, ..
            } => self.infer_set(lhs, rhs, *range),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                range,
            } => {
                let cond_ty = self.infer_expression(cond);
                self.expect_subtype_for(&cond_ty, &boolean_type(), cond.range(), "if-condition");
                let then_ty = self.infer_expression(then_branch);
                let else_ty = self.infer_expression(else_branch);
                self.common_type(&then_ty, &else_ty).unwrap_or_else(|| {
                    self.emit(
                        TYPE_ERROR,
                        format!(
                            "if branches have incompatible types `{then_ty:?}` and `{else_ty:?}`"
                        ),
                        *range,
                    );
                    Type::Any
                })
            }
            Expression::Let {
                name, value, body, ..
            } => {
                let value_ty = self.infer_expression(value);
                self.push_scope();
                self.declare_variable(QNameKey::from_qname(name), value_ty);
                let body_ty = self.infer_expression(body);
                self.pop_scope();
                body_ty
            }
            Expression::For {
                var, source, body, ..
            } => {
                let source_ty = self.infer_expression(source);
                let item_ty = stream_item_type(&source_ty).unwrap_or(source_ty);
                self.push_scope();
                self.declare_variable(QNameKey::from_qname(var), item_ty);
                let body_ty = self.infer_expression(body);
                self.pop_scope();
                Type::stream(body_ty)
            }
            Expression::Quantified {
                var,
                source,
                predicate,
                ..
            } => {
                let source_ty = self.infer_expression(source);
                let item_ty = stream_item_type(&source_ty).unwrap_or(source_ty);
                self.push_scope();
                self.declare_variable(QNameKey::from_qname(var), item_ty);
                let predicate_ty = self.infer_expression(predicate);
                self.expect_subtype(&predicate_ty, &boolean_type(), predicate.range());
                self.pop_scope();
                boolean_type()
            }
            Expression::Record { entries, .. } => {
                let mut fields = Vec::new();
                let mut dynamic_shape = false;
                for entry in entries {
                    match &entry.key {
                        crate::parser::RecordKey::Static { value, .. } => {
                            fields.push(RecordField {
                                name: value.clone(),
                                ty: self.infer_expression(&entry.value),
                            });
                        }
                        crate::parser::RecordKey::Computed { expr, .. } => {
                            dynamic_shape = true;
                            let key_ty = self.infer_expression(expr);
                            self.expect_subtype(
                                &key_ty,
                                &Type::atom(AtomType::String),
                                expr.range(),
                            );
                            self.infer_expression(&entry.value);
                        }
                    }
                }
                if dynamic_shape {
                    Type::Any
                } else {
                    Type::Record(fields)
                }
            }
            Expression::Sequence { items, .. } if items.is_empty() => Type::Empty,
            Expression::Sequence { items, .. } => {
                let mut item_ty = Type::Empty;
                for item in items {
                    let next_ty = self.infer_expression(item);
                    item_ty = if matches!(item_ty, Type::Empty) {
                        next_ty
                    } else {
                        self.common_type(&item_ty, &next_ty).unwrap_or(Type::Any)
                    };
                }
                Type::stream(item_ty)
            }
            Expression::Call {
                callee,
                args,
                range,
            } => self.infer_call(callee, args, *range),
            Expression::Lambda { params, body, .. } => self.infer_lambda(params, body),
            Expression::InstanceOf { value, ty, .. } => {
                self.infer_expression(value);
                self.resolve_type_expr(ty);
                boolean_type()
            }
            Expression::CastAs { value, ty, range } => {
                let value_ty = self.infer_expression(value);
                let target = self.resolve_type_expr(ty);
                if !value_ty.is_any() && !target.is_any() && !can_cast(&value_ty, &target) {
                    self.emit(
                        TYPE_ERROR,
                        format!("cannot cast `{value_ty:?}` as `{target:?}`"),
                        *range,
                    );
                }
                target
            }
            Expression::TreatAs { value, ty, range } => {
                let value_ty = self.infer_expression(value);
                let target = self.resolve_type_expr(ty);
                if !self.expect_subtype(&value_ty, &target, *range) {
                    Type::Any
                } else {
                    target
                }
            }
        }
    }

    fn infer_literal(&self, value: &LiteralValue) -> Type {
        match value {
            LiteralValue::String(_) => Type::atom(AtomType::String),
            LiteralValue::Integer(_) => Type::atom(AtomType::Integer),
            LiteralValue::Decimal(_) => Type::atom(AtomType::Decimal),
            LiteralValue::Double(_) => Type::atom(AtomType::Double),
            LiteralValue::Boolean(_) => boolean_type(),
            LiteralValue::Null => Type::Empty,
        }
    }

    fn infer_path(&mut self, steps: &[PathStep]) -> Type {
        for step in steps {
            if let PathStep::Axis { predicates, .. } = step {
                for predicate in predicates {
                    let predicate_ty = self.infer_expression(predicate);
                    self.expect_subtype(&predicate_ty, &boolean_type(), predicate.range());
                }
            }
        }
        Type::stream(Type::Node(NodeKind::Node))
    }

    fn infer_pipeline(&mut self, source: &Expression, steps: &[PipelineStep]) -> Type {
        let mut current = self.infer_expression(source);
        for step in steps {
            current = match step {
                PipelineStep::Named { name, args, range } => {
                    let any_receiver = current.is_any();
                    let mut all_args = Vec::with_capacity(args.len() + 1);
                    all_args.push(current);
                    all_args.extend(args.iter().map(|arg| self.infer_expression(arg)));
                    if any_receiver
                        && self
                            .lookup_function_signature(name, all_args.len())
                            .is_none()
                    {
                        Type::Any
                    } else {
                        self.call_named(name, &all_args, *range)
                    }
                }
                PipelineStep::Lambda { lambda, .. } => {
                    self.infer_expression(lambda);
                    current
                }
            };
        }
        current
    }

    fn infer_lambda(&mut self, params: &[FunctionParam], body: &Expression) -> Type {
        self.push_scope();
        let param_types: Vec<Type> = params
            .iter()
            .map(|param| {
                let ty = self.param_type(param);
                self.declare_variable(QNameKey::from_qname(&param.name), ty.clone());
                ty
            })
            .collect();
        let ret = self.infer_expression(body);
        self.pop_scope();
        Type::Lambda {
            params: param_types,
            ret: Box::new(ret),
        }
    }

    fn infer_binary(
        &mut self,
        op: BinaryOp,
        lhs: &Expression,
        rhs: &Expression,
        range: ByteRange,
    ) -> Type {
        let lhs_ty = self.infer_expression(lhs);
        let rhs_ty = self.infer_expression(rhs);
        match op {
            BinaryOp::And | BinaryOp::Or => {
                self.expect_subtype(&lhs_ty, &boolean_type(), lhs.range());
                self.expect_subtype(&rhs_ty, &boolean_type(), rhs.range());
                boolean_type()
            }
            BinaryOp::Plus | BinaryOp::Star | BinaryOp::Slash | BinaryOp::Percent => {
                self.infer_numeric_binary(op, &lhs_ty, &rhs_ty, range)
            }
            BinaryOp::Minus => self.infer_minus(&lhs_ty, &rhs_ty, range),
            BinaryOp::Is => {
                if !lhs_ty.is_any() {
                    self.expect_subtype(&lhs_ty, &Type::Node(NodeKind::Node), lhs.range());
                }
                if !rhs_ty.is_any() {
                    self.expect_subtype(&rhs_ty, &Type::Node(NodeKind::Node), rhs.range());
                }
                boolean_type()
            }
            BinaryOp::Coalesce => {
                // `lhs ?? rhs` yields either operand, so the static type is their union.
                self.common_type(&lhs_ty, &rhs_ty).unwrap_or(Type::Any)
            }
            BinaryOp::EqEq
            | BinaryOp::BangEq
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                self.warn_cross_type_compare(&lhs_ty, &rhs_ty, range);
                boolean_type()
            }
        }
    }

    fn infer_numeric_binary(
        &mut self,
        op: BinaryOp,
        lhs: &Type,
        rhs: &Type,
        range: ByteRange,
    ) -> Type {
        if lhs.is_any() || rhs.is_any() {
            return Type::Any;
        }
        if lhs.is_numeric_atom() && rhs.is_numeric_atom() {
            if lhs == rhs {
                return lhs.clone();
            }
            self.emit(
                TYPE_ERROR,
                format!(
                    "operator `{}` requires matching numeric operand types, got `{lhs:?}` and `{rhs:?}`; use an explicit num:* conversion",
                    op.symbol()
                ),
                range,
            );
            return Type::Any;
        }
        self.emit(
            TYPE_ERROR,
            format!(
                "operator `{}` requires numeric operands, got `{lhs:?}` and `{rhs:?}`",
                op.symbol()
            ),
            range,
        );
        Type::Any
    }

    fn infer_minus(&mut self, lhs: &Type, rhs: &Type, range: ByteRange) -> Type {
        if lhs.is_any() || rhs.is_any() {
            return Type::Any;
        }
        if lhs.is_numeric_atom() && rhs.is_numeric_atom() {
            if lhs == rhs {
                return lhs.clone();
            }
            self.emit(
                TYPE_ERROR,
                format!(
                    "operator `-` requires matching numeric operand types, got `{lhs:?}` and `{rhs:?}`; use an explicit num:* conversion"
                ),
                range,
            );
            return Type::Any;
        }
        if lhs.is_stream_like() && rhs.is_stream_like() {
            let lhs_item = stream_item_type(lhs).unwrap_or(Type::Empty);
            let rhs_item = stream_item_type(rhs).unwrap_or(Type::Empty);
            return Type::stream(self.common_type(&lhs_item, &rhs_item).unwrap_or(Type::Any));
        }
        if (lhs.is_numeric_atom() && rhs.is_stream_like())
            || (lhs.is_stream_like() && rhs.is_numeric_atom())
        {
            self.emit(
                TYPE_ERROR,
                format!(
                    "operator `-` cannot mix numeric and stream operands: `{lhs:?}` and `{rhs:?}`"
                ),
                range,
            );
            return Type::Any;
        }
        self.emit(
            TYPE_ERROR,
            format!(
                "operator `-` requires numeric or stream operands, got `{lhs:?}` and `{rhs:?}`"
            ),
            range,
        );
        Type::Any
    }

    fn infer_unary(&mut self, op: UnaryOp, operand: &Expression, range: ByteRange) -> Type {
        let operand_ty = self.infer_expression(operand);
        match op {
            UnaryOp::Not => {
                self.expect_subtype(&operand_ty, &boolean_type(), operand.range());
                boolean_type()
            }
            UnaryOp::Negate => {
                if operand_ty.is_any() || operand_ty.is_numeric_atom() {
                    operand_ty
                } else {
                    self.emit(
                        TYPE_ERROR,
                        format!("numeric negation cannot be applied to `{operand_ty:?}`"),
                        range,
                    );
                    Type::Any
                }
            }
        }
    }

    fn infer_set(&mut self, lhs: &Expression, rhs: &Expression, range: ByteRange) -> Type {
        let lhs_ty = self.infer_expression(lhs);
        let rhs_ty = self.infer_expression(rhs);
        let lhs_item = set_operand_item_type(&lhs_ty);
        let rhs_item = set_operand_item_type(&rhs_ty);

        Type::stream(self.common_type(&lhs_item, &rhs_item).unwrap_or_else(|| {
            self.warn_cross_type_compare(&lhs_item, &rhs_item, range);
            Type::Any
        }))
    }

    fn infer_call(&mut self, callee: &Expression, args: &[Expression], range: ByteRange) -> Type {
        let arg_types: Vec<Type> = args.iter().map(|arg| self.infer_expression(arg)).collect();
        if let Expression::Name(name, _) = callee {
            return self.call_named(name, &arg_types, range);
        }

        match self.infer_expression(callee) {
            Type::Lambda { params, ret } => {
                self.check_call_args(&params, &arg_types, range);
                *ret
            }
            Type::Any => Type::Any,
            other => {
                self.emit(
                    TYPE_ERROR,
                    format!("callee is not callable: `{other:?}`"),
                    range,
                );
                Type::Any
            }
        }
    }

    fn call_named(&mut self, name: &QName, args: &[Type], range: ByteRange) -> Type {
        let Some(signature) = self.lookup_function_signature(name, args.len()) else {
            if self.is_imported_name(name) {
                return Type::Any;
            }
            self.emit(
                UNKNOWN_FUNCTION,
                format!(
                    "unknown function `{}` with arity {}",
                    QNameKey::from_qname(name).display(),
                    args.len()
                ),
                range,
            );
            return Type::Any;
        };
        self.check_call_args(&signature.params, args, range);
        signature.ret
    }

    fn lookup_function_signature(&self, name: &QName, arity: usize) -> Option<FunctionSignature> {
        self.functions
            .get(&FunctionSignatureKey {
                name: QNameKey::from_qname(name),
                arity: Arity(arity.try_into().unwrap_or(u32::MAX)),
            })
            .cloned()
    }

    fn check_call_args(&mut self, params: &[Type], args: &[Type], range: ByteRange) {
        for (actual, expected) in args.iter().zip(params) {
            self.expect_subtype(actual, expected, range);
        }
    }

    fn resolve_type_expr(&mut self, ty: &TypeExpr) -> Type {
        let key = QNameKey::from_qname(&ty.name);
        if let Some(builtin) = builtin_type(&key) {
            return builtin;
        }
        if let Some(schema_id) = self.schemas.resolve(&key) {
            return Type::SchemaElement(schema_id);
        }
        if self.is_imported_key(&key) {
            return Type::Any;
        }
        self.emit(
            UNKNOWN_TYPE,
            format!("unknown type `{}`", key.display()),
            ty.range,
        );
        Type::Any
    }

    fn lookup_variable(&self, name: &QName) -> Option<Type> {
        let key = QNameKey::from_qname(name);
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&key).cloned())
    }

    fn expect_subtype(&mut self, actual: &Type, expected: &Type, range: ByteRange) -> bool {
        self.expect_subtype_for(actual, expected, range, "subtype-check")
    }

    fn expect_subtype_for(
        &mut self,
        actual: &Type,
        expected: &Type,
        range: ByteRange,
        expression_kind: &'static str,
    ) -> bool {
        if actual.is_any() {
            return true;
        }
        if actual.is_subtype_of(expected, &self.schemas) {
            return true;
        }
        self.emit_type_error(
            format!("type `{actual:?}` is not a subtype of `{expected:?}`"),
            range,
            expression_kind,
            Some(actual),
            Some(expected),
        );
        false
    }

    fn common_type(&self, left: &Type, right: &Type) -> Option<Type> {
        if left.is_subtype_of(right, &self.schemas) {
            Some(right.clone())
        } else if right.is_subtype_of(left, &self.schemas) {
            Some(left.clone())
        } else {
            None
        }
    }

    fn warn_cross_type_compare(&mut self, left: &Type, right: &Type, range: ByteRange) {
        if !cross_type_compare(left, right) {
            return;
        }
        self.emit(
            CROSS_TYPE_COMPARE,
            format!("comparison between distinct static types `{left:?}` and `{right:?}`"),
            range,
        );
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn current_scope_mut(&mut self) -> &mut HashMap<QNameKey, Type> {
        if self.scopes.is_empty() {
            self.scopes.push(HashMap::new());
        }
        self.scopes.last_mut().expect("scope stack is not empty")
    }

    fn emit(&mut self, code: DiagnosticCode, message: impl Into<String>, range: ByteRange) {
        let Some(severity) = self.config.severity_for(code) else {
            return;
        };
        let mut diagnostic = diagnostic(code, message, range, severity);
        if code == TYPE_ERROR {
            diagnostic.details = Some(type_error_details("static-expression", None, None, range));
        }
        self.diagnostics.push(diagnostic);
    }

    fn emit_type_error(
        &mut self,
        message: impl Into<String>,
        range: ByteRange,
        expression_kind: &'static str,
        actual: Option<&Type>,
        expected: Option<&Type>,
    ) {
        let Some(severity) = self.config.severity_for(TYPE_ERROR) else {
            return;
        };
        let mut diagnostic = diagnostic(TYPE_ERROR, message, range, severity);
        diagnostic.details = Some(type_error_details(expression_kind, actual, expected, range));
        self.diagnostics.push(diagnostic);
    }

    fn register_import_surface(&mut self, import: &ImportDecl) {
        let Some(alias) = import.alias.as_deref() else {
            return;
        };
        self.imported_prefixes.insert(alias.to_owned());
        if import.uri.starts_with("cem:stdlib/") {
            self.register_stdlib_module_alias(alias, &import.uri);
        }
    }

    fn register_default_stdlib_aliases(&mut self) {
        for (alias, module) in [
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
        ] {
            self.imported_prefixes.insert(alias.to_owned());
            self.register_stdlib_module_alias(alias, module);
        }
    }

    fn register_bare_stdlib_helpers(&mut self) {
        self.register_function(FunctionSignature {
            name: QNameKey::new(None, "same_node"),
            params: vec![Type::Any, Type::Any],
            ret: boolean_type(),
        });
        for function in crate::stdlib::ModuleRegistry::with_all_known().functions {
            let bare = function.module == "cem:stdlib/sequence"
                || (function.module == "cem:stdlib/content-types" && function.name == "read");
            if bare {
                self.register_any_function(
                    QNameKey::new(None, function.name),
                    function.min_arity,
                    function.max_arity,
                );
            }
        }
    }

    fn register_stdlib_module_alias(&mut self, alias: &str, module: &str) {
        for function in crate::stdlib::ModuleRegistry::with_all_known()
            .functions
            .into_iter()
            .filter(|function| function.module == module)
        {
            self.register_any_function(
                QNameKey::new(Some(alias.to_owned()), function.name),
                function.min_arity,
                function.max_arity,
            );
        }
    }

    fn register_any_function(&mut self, name: QNameKey, min_arity: u8, max_arity: u8) {
        for arity in min_arity..=max_arity {
            self.register_function(FunctionSignature {
                name: name.clone(),
                params: vec![Type::Any; arity as usize],
                ret: Type::Any,
            });
        }
    }

    fn is_imported_name(&self, name: &QName) -> bool {
        self.is_imported_key(&QNameKey::from_qname(name))
    }

    fn is_imported_key(&self, key: &QNameKey) -> bool {
        key.prefix
            .as_ref()
            .is_some_and(|prefix| self.imported_prefixes.contains(prefix))
    }
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

fn boolean_type() -> Type {
    Type::atom(AtomType::Boolean)
}

pub(crate) fn stream_item_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::Stream(item) => Some(item.as_ref().clone()),
        Type::Empty => Some(Type::Empty),
        _ => None,
    }
}

fn set_operand_item_type(ty: &Type) -> Type {
    stream_item_type(ty).unwrap_or_else(|| ty.clone())
}

fn can_cast(actual: &Type, expected: &Type) -> bool {
    matches!((actual, expected), (Type::Atom(_), Type::Atom(_)))
        || actual == expected
        || matches!(actual, Type::Empty)
        || matches!(expected, Type::Any)
}

fn cross_type_compare(left: &Type, right: &Type) -> bool {
    match (left, right) {
        (Type::Any, _) | (_, Type::Any) | (Type::Empty, _) | (_, Type::Empty) => false,
        (Type::Atom(a), Type::Atom(b)) => a != b,
        (Type::Atom(_), Type::Stream(_) | Type::Array(_) | Type::Record(_))
        | (Type::Stream(_) | Type::Array(_) | Type::Record(_), Type::Atom(_)) => true,
        (Type::Stream(_), Type::Array(_) | Type::Record(_))
        | (Type::Array(_), Type::Stream(_) | Type::Record(_))
        | (Type::Record(_), Type::Stream(_) | Type::Array(_)) => true,
        _ => false,
    }
}

fn diagnostic(
    code: DiagnosticCode,
    message: impl Into<String>,
    range: ByteRange,
    severity: Severity,
) -> Diagnostic {
    ql_diagnostics::spanned(code, message, range, severity)
}

fn type_error_details(
    expression_kind: &'static str,
    actual: Option<&Type>,
    expected: Option<&Type>,
    range: ByteRange,
) -> serde_json::Value {
    json!({
        "behavior": "cem-ql-type-report-fact",
        "factKind": "type-error",
        "contract": "static-type-check",
        "phase": "static",
        "disposition": "artifact-blocking",
        "recoverable": true,
        "fatal": false,
        "expressionKind": expression_kind,
        "actualType": actual.map(|ty| format!("{ty:?}")),
        "expectedType": expected.map(|ty| format!("{ty:?}")),
        "range": range,
    })
}
