//! Layer 5: typed query IR.

use std::collections::HashMap;

use cem_ml::source_map::SourceMapStack;

use crate::parser::{Axis, BinaryOp, NameTest, QName, QuantifierKind, SetOp, UnaryOp};
use crate::resolve::{BindingId, ModuleUri, SchemaTypeId, StateSlotId, TemplateRefId};
use crate::types::Type;

pub mod deserialize;
pub mod lower;
pub mod serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IrId(pub u32);

#[derive(Debug, Clone)]
pub struct IrTree {
    pub nodes: Vec<IrNode>,
    pub root: IrId,
    pub source_maps: Vec<SourceMapStack>,
    pub types: Vec<Type>,
    pub resolutions: Vec<Option<BindingId>>,
}

impl IrTree {
    pub fn node(&self, id: IrId) -> Option<&IrNode> {
        self.nodes.get(id.0 as usize)
    }

    pub fn ty(&self, id: IrId) -> Option<&Type> {
        self.types.get(id.0 as usize)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrNode {
    LitString(String),
    LitInt(i64),
    LitDecimal(String),
    LitDouble(f64),
    LitBool(bool),
    LitNull,
    LocalVar(BindingId),
    FunctionRef(BindingId),
    SchemaType(SchemaTypeId),
    TemplateRef(TemplateRefId),
    StateSlot(StateSlotId),
    Record(Vec<(String, IrId)>),
    Array(Vec<IrId>),
    Sequence(Vec<IrId>),
    Lambda {
        params: Vec<(BindingId, Type)>,
        body: IrId,
        captures: Vec<BindingId>,
    },
    AxisStep {
        axis: Axis,
        name_test: NameTest,
        predicates: Vec<IrId>,
    },
    Parent,
    Self_,
    Reference,
    Pipeline {
        source: IrId,
        steps: Vec<IrStep>,
    },
    LeadingDot,
    Call {
        callee: IrId,
        args: Vec<IrId>,
    },
    StdlibCall {
        module: ModuleUri,
        name: QName,
        args: Vec<IrId>,
    },
    BinaryOp {
        op: BinaryOp,
        lhs: IrId,
        rhs: IrId,
    },
    UnaryOp {
        op: UnaryOp,
        operand: IrId,
    },
    SetOp {
        op: SetOp,
        lhs: IrId,
        rhs: IrId,
    },
    If {
        cond: IrId,
        then_branch: IrId,
        else_branch: IrId,
    },
    Let {
        name: BindingId,
        value: IrId,
        body: IrId,
    },
    For {
        var: BindingId,
        source: IrId,
        body: IrId,
    },
    Quantified {
        kind: QuantifierKind,
        var: BindingId,
        source: IrId,
        predicate: IrId,
    },
    InstanceOf {
        value: IrId,
        ty: Type,
    },
    CastAs {
        value: IrId,
        ty: Type,
    },
    TreatAs {
        value: IrId,
        ty: Type,
    },
    Is {
        lhs: IrId,
        rhs: IrId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrStep {
    Named {
        name: QName,
        binding: Option<BindingId>,
        args: Vec<IrId>,
    },
    NamedStdlib {
        module: ModuleUri,
        name: QName,
        args: Vec<IrId>,
    },
    Lambda(IrId),
}

#[derive(Debug, Clone)]
pub struct CompiledQuery {
    pub tree: IrTree,
    pub policy_bindings: HashMap<BindingId, String>,
    pub source: String,
}
