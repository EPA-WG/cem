//! Immutable collection methods over retained native values. Adjacent streaming
//! stages pass one item at a time, so a terminal can stop the underlying axis.
use super::{pipeline, AtomValue, BudgetAxis, EvalCtx, Item, ItemStream, QueryItemViewKind};
use crate::{
    ir::{IrId, IrStep},
    parser::QName,
};

use crate::stdlib::dom::chain_method_arity as arity;

pub(super) fn method(
    input: ItemStream,
    name: &QName,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    if input.chain
        || arity(&name.local).is_some()
        || input.items.iter().any(|item| {
            item.view()
                .is_some_and(|v| v.kind() == QueryItemViewKind::Node)
        })
    {
        let scalar = !input.chain && matches!(name.local.as_str(), "name" | "text");
        let mut result = apply(
            input,
            &[IrStep::Method {
                name: name.clone(),
                args: args.to_vec(),
            }],
            ctx,
        );
        if scalar {
            result.chain = false;
        }
        return result;
    }
    // Preserve the established sequence pipeline helpers. Other explicit calls
    // must not silently become record-field reads.
    if matches!(
        name.local.as_str(),
        "first" | "last" | "take" | "drop" | "nth" | "where" | "target"
    ) {
        return pipeline::apply_builtin_step(input, name, args, ctx);
    }
    ctx.type_error(
        args.first().copied().unwrap_or(IrId(0)),
        "method requires a chain or native node receiver",
    )
}

struct Stage {
    name: String,
    args: Vec<ItemStream>,
    source: IrId,
    seen: usize,
    done: bool,
}

impl Stage {
    fn callback(&self) -> IrId {
        match self.args[0].items[0] {
            Item::Lambda(id) => id,
            _ => unreachable!("validated callback"),
        }
    }
    fn limit(&self) -> usize {
        match self.args[0].items[0].atom() {
            Some(AtomValue::Integer(n)) => usize::try_from(n).unwrap_or(usize::MAX),
            _ => unreachable!("validated count"),
        }
    }
}

fn prepare(name: &QName, ids: &[IrId], ctx: &mut EvalCtx<'_>) -> Result<Stage, ItemStream> {
    let source = ids.first().copied().unwrap_or(IrId(0));
    let Some((min, max)) = arity(&name.local) else {
        return Err(ctx.unknown_function(source, "unknown chain method"));
    };
    if ids.len() < min || ids.len() > max {
        return Err(ctx.type_error(source, "invalid chain method arity"));
    }
    let args = ctx.eval_arg_streams(ids);
    if let Some(error) = args.iter().find(|arg| arg.error.is_some()) {
        return Err(error.clone());
    }
    if matches!(
        name.local.as_str(),
        "closest"
            | "find"
            | "rfind"
            | "filter"
            | "map"
            | "flat_map"
            | "any"
            | "all"
            | "sorted_by_key"
    ) {
        match args[0].items.as_slice() {
            [Item::Lambda(id)] if matches!(ctx.query.tree.node(*id), Some(crate::ir::IrNode::Lambda { params, .. }) if params.len() == 1) => {
                ()
            }
            _ => return Err(ctx.type_error(source, "chain callback must accept one argument")),
        }
    }
    if matches!(name.local.as_str(), "take" | "skip" | "nth")
        && !matches!(args[0].items.as_slice(), [item] if matches!(item.atom(), Some(AtomValue::Integer(n)) if n >= 0))
    {
        return Err(ctx.type_error(source, "chain count must be one nonnegative integer"));
    }
    if name.local == "split"
        && !matches!(args[0].items.as_slice(), [item] if matches!(item.atom(), Some(AtomValue::String(_))))
    {
        return Err(ctx.type_error(source, "split separator must be one string"));
    }
    if name.local == "attribute" {
        pipeline::attribute_selector(&args[0], ctx, source)?;
    }
    Ok(Stage {
        name: name.local.clone(),
        args,
        source,
        seen: 0,
        done: false,
    })
}

pub(super) fn apply(mut input: ItemStream, steps: &[IrStep], ctx: &mut EvalCtx<'_>) -> ItemStream {
    if input.error.is_some() {
        input.items.clear();
        return input;
    }
    let mut stages = Vec::new();
    let mut consumed = 0;
    let mut buffered = None;
    for step in steps {
        let IrStep::Method { name, args } = step else {
            break;
        };
        let stage = match prepare(name, args, ctx) {
            Ok(stage) => stage,
            Err(error) => return error,
        };
        consumed += 1;
        if matches!(
            name.local.as_str(),
            "last" | "rfind" | "sorted" | "sorted_by_key" | "rev"
        ) {
            buffered = Some(stage);
            break;
        }
        let terminal = matches!(name.local.as_str(), "any" | "all" | "is_empty" | "count");
        stages.push(stage);
        if terminal {
            break;
        }
    }
    let mut out = ItemStream::empty();
    out.chain = true;
    out.diagnostics.append(&mut input.diagnostics);
    let result: Result<(), ItemStream> = (|| {
        // `take(0)` must not enumerate its input axis.
        if !stages
            .iter()
            .any(|stage| stage.name == "take" && stage.limit() == 0)
        {
            for item in input.items {
                if push(item, &mut stages, &mut out, ctx)? {
                    break;
                }
            }
        }
        if let Some(last) = stages.last() {
            let atom = match last.name.as_str() {
                "any" => Some(AtomValue::Boolean(last.done)),
                "all" => Some(AtomValue::Boolean(!last.done)),
                "is_empty" => Some(AtomValue::Boolean(last.seen == 0)),
                "count" => Some(AtomValue::Integer(last.seen as i64)),
                _ => None,
            };
            if let Some(atom) = atom {
                out.items = vec![Item::Atomic(atom)];
                out.chain = false;
            }
        }
        ctx.force_safe_point(IrId(0))?;
        Ok(())
    })();
    if let Err(mut error) = result {
        error.items.clear();
        return error;
    }
    if let Some(stage) = buffered {
        out = buffer(out, stage, ctx);
    }
    if consumed < steps.len() {
        pipeline::apply_pipeline(out, &steps[consumed..], ctx)
    } else {
        out
    }
}

fn predicate(
    stage: &Stage,
    item: Item,
    ctx: &mut EvalCtx<'_>,
    out: &mut ItemStream,
) -> Result<bool, ItemStream> {
    let mut value = ctx.invoke_lambda(stage.callback(), vec![ItemStream::once(item)]);
    if value.error.is_some() {
        value.items.clear();
        return Err(value);
    }
    out.diagnostics.append(&mut value.diagnostics);
    match value.items.as_slice() {
        [item] => match item.atom() {
            Some(AtomValue::Boolean(value)) => Ok(value),
            _ => Err(ctx.type_error(stage.source, "chain predicate must return one boolean")),
        },
        _ => Err(ctx.type_error(stage.source, "chain predicate must return one boolean")),
    }
}

fn lexical(item: &Item, field: &str) -> Option<String> {
    let values = item.view()?.field(field)?;
    match values.as_slice() {
        [item] => match item.atom()? {
            AtomValue::String(s) => Some(s),
            _ => None,
        },
        _ => None,
    }
}

// A scope-bearing native operation must succeed before inspecting metadata.
fn check_node(item: &Item, ctx: &mut EvalCtx<'_>, source: IrId) -> Result<(), ItemStream> {
    let Some(view) = item.view().filter(|v| v.kind() == QueryItemViewKind::Node) else {
        return Err(ctx.type_error(source, "chain navigation requires native nodes"));
    };
    view.parent(ctx.query_scope)
        .map_err(|e| pipeline::node_access_error(e, ctx, source))?;
    Ok(())
}

/// Returns true when upstream can stop. Recursion is bounded by query stage
/// count; upward traversal uses a loop rather than host-stack recursion.
fn push(
    item: Item,
    stages: &mut [Stage],
    out: &mut ItemStream,
    ctx: &mut EvalCtx<'_>,
) -> Result<bool, ItemStream> {
    if stages.is_empty() {
        ctx.charge_items(1, IrId(0))?;
        out.items.push(item);
        return Ok(false);
    }
    ctx.enter_call(stages[0].source)?;
    let result = push_stage(item, stages, out, ctx);
    ctx.exit_call();
    result
}

fn push_stage(
    item: Item,
    stages: &mut [Stage],
    out: &mut ItemStream,
    ctx: &mut EvalCtx<'_>,
) -> Result<bool, ItemStream> {
    let (stage, rest) = stages.split_first_mut().expect("nonempty stages");
    ctx.charge(BudgetAxis::XPathWorkUnits, 1, stage.source)?;
    let emit = |item, rest: &mut [Stage], out: &mut ItemStream, ctx: &mut EvalCtx<'_>| {
        push(item, rest, out, ctx)
    };
    match stage.name.as_str() {
        "any" | "all" => {
            let passed = predicate(stage, item, ctx, out)?;
            stage.done = if stage.name == "all" { !passed } else { passed };
            Ok(stage.done)
        }
        "is_empty" | "count" => {
            stage.seen += 1;
            Ok(stage.name == "is_empty")
        }
        "nth" => {
            if stage.seen < stage.limit() {
                stage.seen += 1;
                return Ok(false);
            }
            emit(item, rest, out, ctx)?;
            Ok(true)
        }
        "split" => {
            let Some(AtomValue::String(value)) = item.atom() else {
                return Err(ctx.type_error(stage.source, "split requires strings"));
            };
            let Some(AtomValue::String(separator)) = stage.args[0].items[0].atom() else {
                unreachable!("validated separator")
            };
            for part in value.split(separator.as_str()) {
                if emit(Item::Atomic(AtomValue::String(part.to_owned())), rest, out, ctx)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "next" | "take" | "skip" => {
            stage.seen += 1;
            if stage.name == "skip" && stage.seen <= stage.limit() {
                return Ok(false);
            }
            let stopped = emit(item, rest, out, ctx)?;
            Ok(stopped
                || stage.name == "next"
                || (stage.name == "take" && stage.seen >= stage.limit()))
        }
        "filter" | "find" => {
            if !predicate(stage, item.clone(), ctx, out)? {
                return Ok(false);
            }
            Ok(emit(item, rest, out, ctx)? || stage.name == "find")
        }
        "map" | "flat_map" => {
            let mut result = ctx.invoke_lambda(stage.callback(), vec![ItemStream::once(item)]);
            if result.error.is_some() {
                result.items.clear();
                return Err(result);
            }
            out.diagnostics.append(&mut result.diagnostics);
            if stage.name == "map" {
                // A sequence-valued result is one collection member. Explicit
                // flat_map is required to concatenate it into the outer chain.
                let value = if result.chain || result.items.len() != 1 {
                    Item::Array(result.items)
                } else {
                    result.items.remove(0)
                };
                emit(value, rest, out, ctx)
            } else {
                // A returned chain/sequence already exposes its members.
                // Only a single array-valued result needs its wrapper removed;
                // nested arrays inside a returned chain remain nested.
                let values = if !result.chain && result.items.len() == 1 {
                    match result.items.remove(0) {
                        Item::Array(values) => values,
                        Item::Native(view) if view.kind() == QueryItemViewKind::Array => {
                            view.members().ok_or_else(|| {
                                ctx.type_error(stage.source, "array members unavailable")
                            })?
                        }
                        other => vec![other],
                    }
                } else {
                    result.items
                };
                for value in values {
                    if emit(value, rest, out, ctx)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
        "parent" | "children" | "child_nodes" | "ancestors" | "closest" | "name" | "text"
        | "attribute" => {
            check_node(&item, ctx, stage.source)?;
            if stage.name == "name" {
                return match lexical(&item, "name") {
                    Some(name) => emit(Item::Atomic(AtomValue::String(name)), rest, out, ctx),
                    None => Ok(false),
                };
            }
            if stage.name == "text" {
                let mut value =
                    super::values::text(vec![ItemStream::once(item)], ctx, stage.source);
                if value.error.is_some() {
                    return Err(value);
                }
                out.diagnostics.append(&mut value.diagnostics);
                return emit(value.items.remove(0), rest, out, ctx);
            }
            if matches!(stage.name.as_str(), "ancestors" | "closest") {
                let mut current = Some(item);
                if stage.name == "ancestors" {
                    current = parent(current.as_ref().unwrap(), ctx, stage.source)?;
                }
                while let Some(node) = current {
                    ctx.charge(BudgetAxis::XPathWorkUnits, 1, stage.source)?;
                    check_node(&node, ctx, stage.source)?;
                    if lexical(&node, "kind").as_deref() == Some("element") {
                        if stage.name == "ancestors" {
                            if emit(node.clone(), rest, out, ctx)? {
                                return Ok(true);
                            }
                        } else if predicate(stage, node.clone(), ctx, out)? {
                            return emit(node, rest, out, ctx);
                        }
                    }
                    current = parent(&node, ctx, stage.source)?;
                }
                return Ok(false);
            }
            if stage.name == "parent" {
                return match parent(&item, ctx, stage.source)? {
                    Some(parent) => emit(parent, rest, out, ctx),
                    None => Ok(false),
                };
            }
            if stage.name == "attribute" {
                let selector = pipeline::attribute_selector(&stage.args[0], ctx, stage.source)?;
                let mut attributes = pipeline::navigate_node(
                    &ItemStream::once(item),
                    "attribute",
                    Some(&selector),
                    ctx,
                    stage.source,
                );
                if attributes.error.is_some() {
                    return Err(attributes);
                }
                out.diagnostics.append(&mut attributes.diagnostics);
                for attribute in attributes.items {
                    if emit(attribute, rest, out, ctx)? {
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
            let view = item.view().expect("checked native node");
            let mut children = view
                .children(ctx.query_scope)
                .map_err(|e| pipeline::node_access_error(e, ctx, stage.source))?;
            loop {
                ctx.charge(BudgetAxis::XPathWorkUnits, 1, stage.source)?;
                let Some(child) = children.next() else {
                    break;
                };
                let child = child.map_err(|e| pipeline::node_access_error(e, ctx, stage.source))?;
                check_node(&child, ctx, stage.source)?;
                if stage.name == "children" && lexical(&child, "kind").as_deref() != Some("element")
                {
                    continue;
                }
                if emit(child, rest, out, ctx)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Err(ctx.unknown_function(stage.source, "unknown chain method")),
    }
}

fn parent(item: &Item, ctx: &mut EvalCtx<'_>, source: IrId) -> Result<Option<Item>, ItemStream> {
    ctx.charge(BudgetAxis::XPathWorkUnits, 1, source)?;
    let parent = item
        .view()
        .expect("checked native node")
        .parent(ctx.query_scope)
        .map_err(|e| pipeline::node_access_error(e, ctx, source))?;
    if parent.as_ref().is_some_and(|node| {
        !node
            .view()
            .is_some_and(|v| v.kind() == QueryItemViewKind::Node)
    }) {
        return Err(ctx.type_error(source, "parent navigation returned a non-node value"));
    }
    Ok(parent)
}

fn buffer(mut input: ItemStream, stage: Stage, ctx: &mut EvalCtx<'_>) -> ItemStream {
    match stage.name.as_str() {
        "last" => {
            input.items = input.items.pop().into_iter().collect();
        }
        "rev" => {
            input.items.reverse();
        }
        "rfind" => {
            let mut last = None;
            for item in std::mem::take(&mut input.items).into_iter().rev() {
                match predicate(&stage, item.clone(), ctx, &mut input) {
                    Ok(true) => { last = Some(item); break; },
                    Ok(false) => (),
                    Err(error) => return error,
                }
            }
            input.items = last.into_iter().collect();
        }
        "sorted" | "sorted_by_key" => {
            let mut args = vec![input];
            if stage.name == "sorted" {
                args.push(ItemStream::empty());
            }
            args.extend(stage.args);
            input = pipeline::keyed_collection(
                args,
                ctx,
                &[stage.source],
                false,
                stage.name == "sorted",
            );
        }
        _ => unreachable!(),
    }
    input.chain = true;
    input
}

#[cfg(test)]
mod tests;
