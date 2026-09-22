//! Select an owned field directly from a proven immutable record binding.
use super::{apply_pipeline, step_source};
use crate::eval::{EvalCtx, EvalError, Item, ItemStream};
use crate::ir::{IrId, IrNode, IrStep};
use crate::resolve::BindingId;

fn binding<'ctx>(ctx: &'ctx EvalCtx<'_>, id: BindingId) -> Option<&'ctx ItemStream> {
    ctx.scopes
        .iter()
        .rev()
        .find_map(|scope| scope.get(&id))
        .or_else(|| ctx.borrowed_inputs.get(&id).copied())
}

pub(crate) fn try_project(
    ctx: &mut EvalCtx<'_>,
    source: IrId,
    steps: &[IrStep],
) -> Option<ItemStream> {
    if !ctx.direct_record_reads {
        return None;
    }
    let IrNode::LocalVar(id) = ctx.query.tree.node(source)? else {
        return None;
    };
    let id = *id;
    let IrStep::Named {
        binding: None,
        name,
        args,
    } = steps.first()?
    else {
        return None;
    };
    if name.prefix.is_some()
        || !args.is_empty()
        || matches!(
            name.local.as_str(),
            "first" | "last" | "take" | "drop" | "nth" | "target" | "where"
        )
    {
        return None;
    }
    let input = binding(ctx, id)?;
    // No native accessor runs while an evaluator binding is borrowed. Whole
    // values, arrays/mixed streams, failed inputs and global evaluation fall back.
    if input.error.is_some() || !matches!(input.items.as_slice(), [Item::Record(_)]) {
        return None;
    }
    // Match eval_id(LocalVar), including its source safe point and pending
    // failure rule. The eligible source has no input error or evaluation body.
    if let Err(error) = ctx.ensure_active(source) {
        return Some(apply_pipeline(error, steps, ctx));
    }
    if (ctx.recovery_depth > 0 || matches!(ctx.error, Some(EvalError::Raised { .. })))
        && ctx.error.is_some()
    {
        return Some(apply_pipeline(ctx.pending_failure(), steps, ctx));
    }
    let step_source = step_source(&steps[0]);
    if let Err(error) = ctx.poll_work(step_source) {
        return Some(error);
    }
    let projected = {
        #[cfg(test)]
        let _profile = crate::compile_profile::Span::new("eval/direct-record-read");
        let input = binding(ctx, id).expect("safe points do not change bindings");
        let [Item::Record(fields)] = input.items.as_slice() else {
            unreachable!()
        };
        let items = if let Some(values) = fields.get(&name.local) {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/direct-field-selected");
            values.clone()
        } else {
            Vec::new()
        };
        let mut out = ItemStream::from_items(items);
        out.diagnostics = input.diagnostics.clone();
        out
    };
    // The borrow has ended. Preserve the first field step's acceptance point
    // before dispatching the remaining steps through the ordinary owned path.
    if let Err(error) = ctx.force_safe_point(step_source) {
        return Some(error);
    }
    Some(apply_pipeline(projected, &steps[1..], ctx))
}
