//! Set-operator evaluator.

use std::collections::BTreeSet;

use crate::eval::{item_identity, EvalCtx, Item, ItemStream};
use crate::ir::IrId;
use crate::parser::SetOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetOperatorEvaluator;

pub(crate) fn apply_set_op(
    op: SetOp,
    lhs: ItemStream,
    rhs: ItemStream,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> ItemStream {
    let mut out = ItemStream::empty();
    out.diagnostics.extend(lhs.diagnostics.clone());
    out.diagnostics.extend(rhs.diagnostics.clone());
    out.error = lhs.error.clone().or(rhs.error.clone());
    match op {
        SetOp::Union => union(lhs.items, rhs.items, ctx, source, &mut out),
        SetOp::Intersect => intersect(lhs.items, rhs.items, ctx, source, &mut out),
        SetOp::Difference => difference(lhs.items, rhs.items, ctx, source, &mut out),
        SetOp::SymmetricDifference => {
            symmetric_difference(lhs.items, rhs.items, ctx, source, &mut out)
        }
    }
    out
}

fn union(
    lhs: Vec<Item>,
    rhs: Vec<Item>,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
    out: &mut ItemStream,
) {
    let mut seen = BTreeSet::new();
    for item in lhs.into_iter().chain(rhs) {
        if seen.insert(item_identity(&item)) {
            if let Err(err) = ctx.charge_items(1, source) {
                out.extend_diagnostics(err);
                return;
            }
            out.items.push(item);
        }
    }
}

fn intersect(
    lhs: Vec<Item>,
    rhs: Vec<Item>,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
    out: &mut ItemStream,
) {
    let mut rhs_seen = BTreeSet::new();
    for item in rhs {
        if let Err(err) = ctx.charge_items(1, source) {
            out.extend_diagnostics(err);
            return;
        }
        rhs_seen.insert(item_identity(&item));
    }
    let mut emitted = BTreeSet::new();
    for item in lhs {
        let identity = item_identity(&item);
        if rhs_seen.contains(&identity) && emitted.insert(identity) {
            if let Err(err) = ctx.charge_items(1, source) {
                out.extend_diagnostics(err);
                return;
            }
            out.items.push(item);
        }
    }
}

fn difference(
    lhs: Vec<Item>,
    rhs: Vec<Item>,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
    out: &mut ItemStream,
) {
    let mut rhs_seen = BTreeSet::new();
    for item in rhs {
        if let Err(err) = ctx.charge_items(1, source) {
            out.extend_diagnostics(err);
            return;
        }
        rhs_seen.insert(item_identity(&item));
    }
    let mut emitted = BTreeSet::new();
    for item in lhs {
        let identity = item_identity(&item);
        if !rhs_seen.contains(&identity) && emitted.insert(identity) {
            if let Err(err) = ctx.charge_items(1, source) {
                out.extend_diagnostics(err);
                return;
            }
            out.items.push(item);
        }
    }
}

fn symmetric_difference(
    lhs: Vec<Item>,
    rhs: Vec<Item>,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
    out: &mut ItemStream,
) {
    let mut lhs_seen = BTreeSet::new();
    for item in &lhs {
        if let Err(err) = ctx.charge_items(1, source) {
            out.extend_diagnostics(err);
            return;
        }
        lhs_seen.insert(item_identity(item));
    }
    let mut rhs_seen = BTreeSet::new();
    for item in &rhs {
        if let Err(err) = ctx.charge_items(1, source) {
            out.extend_diagnostics(err);
            return;
        }
        rhs_seen.insert(item_identity(item));
    }
    let mut emitted = BTreeSet::new();
    for item in lhs {
        let identity = item_identity(&item);
        if !rhs_seen.contains(&identity) && emitted.insert(identity) {
            if let Err(err) = ctx.charge_items(1, source) {
                out.extend_diagnostics(err);
                return;
            }
            out.items.push(item);
        }
    }
    for item in rhs {
        let identity = item_identity(&item);
        if !lhs_seen.contains(&identity) && emitted.insert(identity) {
            if let Err(err) = ctx.charge_items(1, source) {
                out.extend_diagnostics(err);
                return;
            }
            out.items.push(item);
        }
    }
}
