//! Selected calls share the ordinary value/callback helpers, with no full
//! catalogue assembly or fallback to the general dispatcher.
use super::*;
use crate::selected_registry::SelectedBuiltin;

const fn link_mask() -> u32 {
    #[cfg(all(target_arch = "wasm32", cem_selected_application))]
    {
        crate::application_runtime::selected_builtin_link_mask()
    }
    #[cfg(not(all(target_arch = "wasm32", cem_selected_application)))]
    {
        u32::MAX
    }
}

#[inline(always)]
const fn linked<const MASK: u32>(entry: SelectedBuiltin) -> bool {
    MASK & (1 << entry as u32) != 0
}

pub(super) fn call(module: &str, name: &str, args: &[IrId], ctx: &mut EvalCtx<'_>) -> ItemStream {
    call_linked::<{ link_mask() }>(module, name, args, ctx)
}

pub(super) fn call_linked<const MASK: u32>(
    module: &str,
    name: &str,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    let source = args.first().copied().unwrap_or(IrId(0));
    let Some(entry) = ctx
        .selected_registry
        .as_ref()
        .and_then(|r| r.resolve(module, name, args.len()))
    else {
        return ctx.unknown_function(source, "function is not in the selected registry");
    };
    if !linked::<MASK>(entry) {
        return ctx.unknown_function(source, "function is not linked in the selected build");
    }
    let values = ctx.eval_arg_streams(args);
    if let Some(failed) = values.iter().find(|stream| stream.error.is_some()) {
        return failed.clone();
    }
    use SelectedBuiltin::*;
    match entry {
        SequenceFirst if linked::<MASK>(SequenceFirst) => {
            pipeline::first(values.into_iter().next().unwrap_or_default())
        }
        SequenceLast if linked::<MASK>(SequenceLast) => {
            pipeline::last(values.into_iter().next().unwrap_or_default())
        }
        SequenceCount if linked::<MASK>(SequenceCount) => ItemStream::once(Item::Atomic(
            AtomValue::Integer(values.into_iter().next().unwrap_or_default().items.len() as i64),
        )),
        SequenceTake if linked::<MASK>(SequenceTake) => {
            let mut values = values.into_iter();
            let input = values.next().unwrap_or_default();
            let n = values.next();
            pipeline::take(input, n.as_ref())
        }
        SequenceDrop if linked::<MASK>(SequenceDrop) => {
            let mut values = values.into_iter();
            let input = values.next().unwrap_or_default();
            let n = values.next();
            pipeline::drop(input, n.as_ref())
        }
        SequenceMap if linked::<MASK>(SequenceMap) => {
            pipeline::callable_sequence(values, ctx, args, false, false)
        }
        SequenceFlatMap if linked::<MASK>(SequenceFlatMap) => {
            pipeline::callable_sequence(values, ctx, args, true, false)
        }
        SequenceWhere if linked::<MASK>(SequenceWhere) => {
            pipeline::callable_sequence(values, ctx, args, false, true)
        }
        SequenceAny if linked::<MASK>(SequenceAny) => {
            pipeline::any_all_sequence(values, ctx, args, false)
        }
        SequenceAll if linked::<MASK>(SequenceAll) => {
            pipeline::any_all_sequence(values, ctx, args, true)
        }
        SequencePeek if linked::<MASK>(SequencePeek) => {
            values.into_iter().next().unwrap_or_default()
        }
        StringLength if linked::<MASK>(StringLength) => ItemStream::once(Item::Atomic(
            AtomValue::Integer(pipeline::first_string(&values).chars().count() as i64),
        )),
        StringLower if linked::<MASK>(StringLower) => ItemStream::once(Item::Atomic(
            AtomValue::String(pipeline::first_string(&values).to_lowercase()),
        )),
        StringUpper if linked::<MASK>(StringUpper) => ItemStream::once(Item::Atomic(
            AtomValue::String(pipeline::first_string(&values).to_uppercase()),
        )),
        StringContains if linked::<MASK>(StringContains) => {
            pipeline::string_predicate(values, |left, right| left.contains(right))
        }
        _ => ctx.unknown_function(source, "function is not linked in the selected build"),
    }
}
/// `Some` handles a selected pipeline, including explicit refusal. `None` leaves
/// local lambdas and ordinary field navigation to the shared evaluator.
pub(super) fn pipeline_step(
    input: ItemStream,
    step: &crate::ir::IrStep,
    ctx: &mut EvalCtx<'_>,
) -> Result<ItemStream, ItemStream> {
    pipeline_step_linked::<{ link_mask() }>(input, step, ctx)
}

pub(super) fn pipeline_step_linked<const MASK: u32>(
    input: ItemStream,
    step: &crate::ir::IrStep,
    ctx: &mut EvalCtx<'_>,
) -> Result<ItemStream, ItemStream> {
    use crate::ir::IrStep;
    let (module, name, args) = match step {
        IrStep::NamedStdlib { module, name, args } => (module.0.as_str(), name, args),
        IrStep::Named {
            binding: None,
            name,
            args,
        } if matches!(
            name.local.as_str(),
            "first" | "last" | "take" | "drop" | "nth" | "where" | "target"
        ) =>
        {
            ("cem:stdlib/sequence", name, args)
        }
        IrStep::Method { args, .. } => {
            return Ok(ctx.unsupported(
                args.first().copied().unwrap_or(IrId(0)),
                "receiver methods need a selected execution contract",
            ))
        }
        IrStep::Named { .. } | IrStep::Lambda(_) => return Err(input),
    };
    let source = args.first().copied().unwrap_or(IrId(0));
    if module != "cem:stdlib/sequence"
        || !matches!(
            name.local.as_str(),
            "first" | "last" | "take" | "drop" | "where"
        )
        || ctx
            .selected_registry
            .as_ref()
            .and_then(|r| r.resolve(module, &name.local, args.len() + 1))
            .is_none()
    {
        return Ok(ctx.unknown_function(source, "pipeline is not in the selected registry"));
    }
    Ok(match name.local.as_str() {
        "first" if linked::<MASK>(SelectedBuiltin::SequenceFirst) => pipeline::first(input),
        "last" if linked::<MASK>(SelectedBuiltin::SequenceLast) => pipeline::last(input),
        "take" if linked::<MASK>(SelectedBuiltin::SequenceTake) => {
            pipeline::take(input, ctx.eval_arg_streams(args).first())
        }
        "drop" if linked::<MASK>(SelectedBuiltin::SequenceDrop) => {
            pipeline::drop(input, ctx.eval_arg_streams(args).first())
        }
        "where" if linked::<MASK>(SelectedBuiltin::SequenceWhere) => {
            pipeline::where_step(input, args.first().copied(), ctx)
        }
        _ => ctx.unknown_function(source, "pipeline is not in the selected registry"),
    })
}
