//! URL call integration: diagnostics use the call IR node, not an argument.
use super::{AtomValue, EvalCtx, Item, ItemStream};
use crate::{
    ir::IrId,
    stdlib::{url, url_params, url_parts},
};
use cem_ml::diagnostics::Severity;

pub(super) fn apply(name: &str, args: &[IrId], ctx: &mut EvalCtx<'_>, source: IrId) -> ItemStream {
    // Like native calls, reports belong to the invoking evaluator, not values.
    // Stop at the first failed argument and retain earlier reports exactly once.
    let mut streams = Vec::new();
    for argument in args {
        let mut stream = ctx.eval_id(*argument);
        for diagnostic in stream.diagnostics.drain(..) {
            if !ctx.diagnostics.contains(&diagnostic) {
                ctx.diagnostics.push(diagnostic);
            }
        }
        if stream.error.is_some() {
            return stream;
        }
        streams.push(stream);
    }
    let mut out = if !url::FUNCTIONS
        .iter()
        .any(|function| function.name == name && function.accepts_arity(args.len()))
    {
        ctx.unknown_function(source, "unknown URL call")
    } else if name == "params" || name.starts_with("params_") {
        let arguments = streams
            .into_iter()
            .map(|stream| stream.items)
            .collect::<Vec<_>>();
        match url_params::call(name, &arguments) {
            Ok(items) => ItemStream::from_items(items),
            Err(error) => ctx.raise(source, "cem.ql.type_error".into(), error.to_string()),
        }
    } else {
        let input = streams[0].items.as_slice();
        let second = streams.get(1).map(|stream| stream.items.as_slice());
        match name {
            "can_parse" => match url::can_parse(input, second) {
                Ok(value) => ItemStream::once(Item::Atomic(AtomValue::Boolean(value))),
                Err(error) => ctx.raise(source, error.code().into(), error.to_string()),
            },
            "parse" => match url::parse(input, second) {
                Ok(Some(item)) => ItemStream::once(item),
                Ok(None) => ItemStream::empty(),
                Err(error) => ctx.raise(source, error.code().into(), error.to_string()),
            },
            "href" => match url::href(input, second) {
                Ok(value) => ItemStream::once(Item::Atomic(value)),
                Err(error) => ctx.raise(source, error.code().into(), error.to_string()),
            },
            "assemble" | "with_parts" => {
                let result = if name == "assemble" {
                    url_parts::assemble(input, second)
                } else {
                    url_parts::with_parts(input, second.expect("validated arity"))
                };
                match result {
                    Ok(result) => {
                        let mut out = ItemStream::once(Item::Atomic(result.href));
                        for warning in result.warnings {
                            out.extend_diagnostics(ctx.emit_diagnostic(
                                source,
                                warning.code(),
                                warning.to_string(),
                                Severity::Warning,
                            ));
                        }
                        out
                    }
                    Err(error) => ctx.raise(source, error.code().into(), error.to_string()),
                }
            }
            _ => unreachable!(),
        }
    };
    // raise/emit already retained these reports on the evaluator context.
    out.diagnostics.clear();
    out
}
