//! Pipeline iterator-chain adapters.

use crate::eval::{effective_boolean, first_integer, AtomValue, EvalCtx, Item, ItemStream};
use crate::ir::{IrId, IrStep};
use crate::resolve::ModuleUri;
use crate::{parser::QName, parser::SetOp};

pub(crate) fn apply_pipeline(
    source: ItemStream,
    steps: &[IrStep],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    let mut stream = source;
    for step in steps {
        stream = apply_step(stream, step, ctx);
        if stream.error.is_some() {
            break;
        }
    }
    stream
}

fn apply_step(input: ItemStream, step: &IrStep, ctx: &mut EvalCtx<'_>) -> ItemStream {
    match step {
        IrStep::Lambda(lambda) => apply_lambda_step(input, *lambda, ctx),
        IrStep::Named {
            binding,
            name,
            args,
        } => {
            let Some(binding) = binding else {
                return apply_builtin_step(input, name, args, ctx);
            };
            let mut out = ItemStream::empty();
            for item in input.items {
                let mut call_args = vec![ItemStream::once(item)];
                call_args.extend(ctx.eval_arg_streams(args));
                let stream = ctx.invoke_function(
                    *binding,
                    call_args,
                    args.first().copied().unwrap_or(IrId(0)),
                );
                out.append_stream(stream);
                if out.error.is_some() {
                    break;
                }
            }
            out
        }
        IrStep::NamedStdlib { module, name, args } => {
            apply_stdlib_step(input, module, name, args, ctx)
        }
    }
}

fn apply_lambda_step(input: ItemStream, lambda: IrId, ctx: &mut EvalCtx<'_>) -> ItemStream {
    let mut out = ItemStream::empty();
    out.diagnostics.extend(input.diagnostics);
    out.error = input.error;
    for item in input.items {
        let stream = ctx.with_current_item(item, |ctx| ctx.invoke_lambda(lambda, Vec::new()));
        out.append_stream(stream);
        if out.error.is_some() {
            break;
        }
    }
    out
}

fn apply_builtin_step(
    input: ItemStream,
    name: &QName,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    match name.local.as_str() {
        "first" => first(input),
        "last" => last(input),
        "take" => take(input, ctx.eval_arg_streams(args).first()),
        "drop" => drop(input, ctx.eval_arg_streams(args).first()),
        "nth" => nth(input, ctx.eval_arg_streams(args).first()),
        "where" => where_step(input, args.first().copied(), ctx),
        _ => ctx.unsupported(
            args.first().copied().unwrap_or(IrId(0)),
            "unknown pipeline step",
        ),
    }
}

fn apply_stdlib_step(
    input: ItemStream,
    module: &ModuleUri,
    name: &QName,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    if module.0 == "cem:stdlib/sequence" {
        return apply_builtin_step(input, name, args, ctx);
    }
    ctx.unsupported(
        args.first().copied().unwrap_or(IrId(0)),
        "unknown stdlib pipeline step",
    )
}

pub(crate) fn apply_stdlib_call(
    module: &ModuleUri,
    name: &QName,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    let arg_streams = ctx.eval_arg_streams(args);
    match (module.0.as_str(), name.local.as_str()) {
        ("cem:stdlib/sequence", "first") => {
            first(arg_streams.into_iter().next().unwrap_or_default())
        }
        ("cem:stdlib/sequence", "last") => last(arg_streams.into_iter().next().unwrap_or_default()),
        ("cem:stdlib/sequence", "take") => {
            let mut args = arg_streams.into_iter();
            let source = args.next().unwrap_or_default();
            let n = args.next();
            take(source, n.as_ref())
        }
        ("cem:stdlib/sequence", "drop") => {
            let mut args = arg_streams.into_iter();
            let source = args.next().unwrap_or_default();
            let n = args.next();
            drop(source, n.as_ref())
        }
        ("cem:stdlib/sequence", "nth") => {
            let mut args = arg_streams.into_iter();
            let source = args.next().unwrap_or_default();
            let n = args.next();
            nth(source, n.as_ref())
        }
        ("cem:stdlib/sequence", "union") => binary_set_call(SetOp::Union, arg_streams, ctx, args),
        ("cem:stdlib/sequence", "intersect") => {
            binary_set_call(SetOp::Intersect, arg_streams, ctx, args)
        }
        ("cem:stdlib/sequence", "difference") => {
            binary_set_call(SetOp::Difference, arg_streams, ctx, args)
        }
        ("cem:stdlib/sequence", "symmetric_difference") => {
            binary_set_call(SetOp::SymmetricDifference, arg_streams, ctx, args)
        }
        ("cem:stdlib/strings", "length") => {
            let value = arg_streams
                .first()
                .and_then(|stream| stream.items.first())
                .and_then(item_string)
                .unwrap_or_default();
            ItemStream::once(Item::Atomic(AtomValue::Integer(
                value.chars().count() as i64
            )))
        }
        ("cem:stdlib/strings", "lower") => {
            let value = arg_streams
                .first()
                .and_then(|stream| stream.items.first())
                .and_then(item_string)
                .unwrap_or_default();
            ItemStream::once(Item::Atomic(AtomValue::String(value.to_lowercase())))
        }
        ("cem:stdlib/strings", "upper") => {
            let value = arg_streams
                .first()
                .and_then(|stream| stream.items.first())
                .and_then(item_string)
                .unwrap_or_default();
            ItemStream::once(Item::Atomic(AtomValue::String(value.to_uppercase())))
        }
        _ => ctx.unsupported(
            args.first().copied().unwrap_or(IrId(0)),
            "unknown stdlib call",
        ),
    }
}

fn first(mut input: ItemStream) -> ItemStream {
    input.items.truncate(1);
    input
}

fn last(mut input: ItemStream) -> ItemStream {
    if let Some(item) = input.items.pop() {
        input.items = vec![item];
    }
    input
}

fn take(mut input: ItemStream, n: Option<&ItemStream>) -> ItemStream {
    let n = n.and_then(first_integer).unwrap_or(0).max(0) as usize;
    input.items.truncate(n);
    input
}

fn drop(mut input: ItemStream, n: Option<&ItemStream>) -> ItemStream {
    let n = n.and_then(first_integer).unwrap_or(0).max(0) as usize;
    input.items = input.items.into_iter().skip(n).collect();
    input
}

fn nth(mut input: ItemStream, n: Option<&ItemStream>) -> ItemStream {
    let n = n.and_then(first_integer).unwrap_or(1).max(1) as usize;
    input.items = input
        .items
        .into_iter()
        .nth(n.saturating_sub(1))
        .into_iter()
        .collect();
    input
}

fn where_step(input: ItemStream, predicate: Option<IrId>, ctx: &mut EvalCtx<'_>) -> ItemStream {
    let Some(predicate) = predicate else {
        return input;
    };
    let mut out = ItemStream::empty();
    out.diagnostics.extend(input.diagnostics);
    out.error = input.error;
    for item in input.items {
        let predicate_stream = ctx.with_current_item(item.clone(), |ctx| ctx.eval_id(predicate));
        if effective_boolean(&predicate_stream.items) {
            out.items.push(item);
        }
        out.extend_diagnostics(predicate_stream);
        if out.error.is_some() {
            break;
        }
    }
    out
}

fn binary_set_call(
    op: SetOp,
    arg_streams: Vec<ItemStream>,
    ctx: &mut EvalCtx<'_>,
    arg_ids: &[IrId],
) -> ItemStream {
    let mut args = arg_streams.into_iter();
    let lhs = args.next().unwrap_or_default();
    let rhs = args.next().unwrap_or_default();
    crate::eval::set_ops::apply_set_op(
        op,
        lhs,
        rhs,
        ctx,
        arg_ids.first().copied().unwrap_or(IrId(0)),
    )
}

fn item_string(item: &Item) -> Option<String> {
    match item {
        Item::Atomic(AtomValue::String(value)) | Item::Atomic(AtomValue::AnyUri(value)) => {
            Some(value.clone())
        }
        Item::Atomic(AtomValue::Integer(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Decimal(value)) => Some(value.clone()),
        Item::Atomic(AtomValue::Double(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Boolean(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Null) => Some("null".to_owned()),
        Item::Node(value) => Some(value.clone()),
        _ => None,
    }
}
