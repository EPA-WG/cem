//! Selected calls share the ordinary value/callback helpers, with no full
//! catalogue assembly or fallback to the general dispatcher.
use super::*;
use crate::selected_registry::SelectedBuiltin;
pub(super) fn call(module: &str, name: &str, args: &[IrId], ctx: &mut EvalCtx<'_>) -> ItemStream {
    let source = args.first().copied().unwrap_or(IrId(0));
    let Some(entry) = ctx
        .selected_registry
        .as_ref()
        .and_then(|r| r.resolve(module, name, args.len()))
    else {
        return ctx.unknown_function(source, "function is not in the selected registry");
    };
    let values = ctx.eval_arg_streams(args);
    if let Some(failed) = values.iter().find(|stream| stream.error.is_some()) {
        return failed.clone();
    }
    use SelectedBuiltin::*;
    match entry {
        SequenceFirst => pipeline::first(values.into_iter().next().unwrap_or_default()),
        SequenceLast => pipeline::last(values.into_iter().next().unwrap_or_default()),
        SequenceCount => ItemStream::once(Item::Atomic(AtomValue::Integer(
            values.into_iter().next().unwrap_or_default().items.len() as i64,
        ))),
        SequenceTake | SequenceDrop => {
            let mut values = values.into_iter();
            let input = values.next().unwrap_or_default();
            let n = values.next();
            if entry == SequenceTake {
                pipeline::take(input, n.as_ref())
            } else {
                pipeline::drop(input, n.as_ref())
            }
        }
        SequenceMap => pipeline::callable_sequence(values, ctx, args, false, false),
        SequenceFlatMap => pipeline::callable_sequence(values, ctx, args, true, false),
        SequenceWhere => pipeline::callable_sequence(values, ctx, args, false, true),
        SequenceAny => pipeline::any_all_sequence(values, ctx, args, false),
        SequenceAll => pipeline::any_all_sequence(values, ctx, args, true),
        SequencePeek => values.into_iter().next().unwrap_or_default(),
        StringLength => ItemStream::once(Item::Atomic(AtomValue::Integer(
            pipeline::first_string(&values).chars().count() as i64,
        ))),
        StringLower => ItemStream::once(Item::Atomic(AtomValue::String(
            pipeline::first_string(&values).to_lowercase(),
        ))),
        StringUpper => ItemStream::once(Item::Atomic(AtomValue::String(
            pipeline::first_string(&values).to_uppercase(),
        ))),
        StringContains => pipeline::string_predicate(values, |left, right| left.contains(right)),
    }
}
/// `Some` handles a selected pipeline, including explicit refusal. `None` leaves
/// local lambdas and ordinary field navigation to the shared evaluator.
pub(super) fn pipeline_step(
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
    Ok(pipeline::apply_builtin_step(input, name, args, ctx))
}
