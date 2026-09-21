use super::*;

pub(super) fn apply(mut args: Vec<ItemStream>, ctx: &mut EvalCtx<'_>, source: IrId) -> ItemStream {
    let mode = args.pop().unwrap_or_default();
    let mode = match mode.items.as_slice() {
        [item]
            if item
                .view()
                .is_none_or(|v| v.kind() == QueryItemViewKind::Atomic) =>
        {
            item.atom()
        }
        _ => None,
    };
    let Some(AtomValue::String(mode)) = mode else {
        return ctx.type_error(source, "cemt:apply_templates requires one mode string");
    };
    if let Err(error) = ctx.enter_call(source) {
        return error;
    }
    let Some(host) = ctx.template_host.take() else {
        ctx.exit_call();
        return ctx.fail_diagnostic(
            source,
            crate::diagnostics::DiagnosticCode("cem.ql.template_context_missing"),
            "cemt:apply_templates requires an active CEMT host",
            "missing template host",
        );
    };
    let result = host.apply_templates(
        args.pop().unwrap_or_default(),
        &mode,
        &ctx.source_map(source),
    );
    ctx.template_host = Some(host);
    ctx.exit_call();
    if let Err(error) = ctx.force_safe_point(source) {
        return error;
    }
    if let Some(error) = &result.error {
        ctx.error = Some(error.clone());
    } else if let Err(error) = ctx.charge_items(result.items.len() as u64, source) {
        return error;
    }
    result
}
