use super::{imported_cem_tree, AtomValue, Item};

/// String import uses the shared native profile resolver. This layer only
/// validates query values and maps typed import failures into query diagnostics.
pub(crate) fn parse(
    arguments: Vec<crate::eval::ItemStream>,
    ctx: &mut crate::eval::EvalCtx<'_>,
    source_id: crate::ir::IrId,
) -> crate::eval::ItemStream {
    use crate::eval::ItemStream;
    use cem_ml::import::{
        import_string, resolve_string_import, ImportFailureKind, ImportStringOption,
        ImportStringRequest,
    };
    let input = match arguments[0].items.as_slice() {
        [] => None,
        [item] => match item.atom() {
            Some(AtomValue::String(text) | AtomValue::AnyUri(text)) => Some(text),
            _ => return ctx.type_error(source_id, "data:parse requires zero or one source string"),
        },
        _ => return ctx.type_error(source_id, "data:parse requires zero or one source string"),
    };
    let [format] = arguments[1].items.as_slice() else {
        return ctx.type_error(source_id, "data:parse requires one format string");
    };
    let Some(AtomValue::String(format)) = format.atom() else {
        return ctx.type_error(source_id, "data:parse requires one format string");
    };
    let mut options = std::collections::BTreeMap::new();
    if let Some(argument) = arguments.get(2) {
        let fields = match argument.items.as_slice() {
            [Item::Record(fields)] => fields.clone(),
            _ => {
                return ctx.type_error(
                    source_id,
                    "data:parse options must be one record of scalar controls",
                )
            }
        };
        for (name, items) in fields {
            let value = match items.as_slice() {
                [item] => match item.atom() {
                    Some(AtomValue::String(text)) => Some(ImportStringOption::String(text)),
                    Some(AtomValue::Boolean(value)) => Some(ImportStringOption::Boolean(value)),
                    _ => None,
                },
                _ => None,
            };
            let Some(value) = value else {
                return ctx.type_error(
                    source_id,
                    "data:parse option values must be single strings or booleans",
                );
            };
            options.insert(name, value);
        }
    }
    let config = match resolve_string_import(&format, &options) {
        Ok(config) => config,
        Err(error) => return ctx.raise(source_id, "cem.ql.import_options".into(), error),
    };
    let Some(input) = input else {
        return ItemStream::empty();
    };
    // Charge work before bounded native parsing, and preserve host cancellation.
    for _ in 0..=input.len().min(cem_ml::import::MAX_BYTES) / 64 {
        if let Err(error) = ctx.poll_work(source_id) {
            return error;
        }
    }
    let result = import_string(ImportStringRequest {
        source: &input,
        source_uri: "data:parse/source",
        base_uri: config.base_uri.as_deref(),
        profile: config.profile,
    });
    if let Err(error) = ctx.force_safe_point(source_id) {
        return error;
    }
    match result {
        Ok(tree) => ItemStream::once(imported_cem_tree(tree)),
        Err(failure) => match failure.kind {
            ImportFailureKind::Malformed | ImportFailureKind::DuplicateKey => {
                let duplicate = failure.kind == ImportFailureKind::DuplicateKey;
                let mut diagnostic = ctx
                    .diagnostic(
                        source_id,
                        if duplicate {
                            "cem.ql.import_duplicate_key"
                        } else {
                            "cem.ql.import_malformed"
                        },
                        failure.message,
                        cem_ml::diagnostics::Severity::Error,
                    )
                    .with_error_name(
                        "urn:cem:import",
                        if duplicate {
                            "duplicate-key"
                        } else {
                            "invalid-source"
                        },
                    );
                // Explicit diagnostic metadata preserves parser locations next
                // to the query location without serializing the imported tree.
                if let Some(details) = diagnostic
                    .details
                    .as_mut()
                    .and_then(|value| value.as_object_mut())
                {
                    details.insert(
                        "importDiagnostics".into(),
                        serde_json::to_value(failure.diagnostics).expect("diagnostic metadata"),
                    );
                }
                ctx.raise_diagnostic(diagnostic)
            }
            ImportFailureKind::Limit => ctx.fail_diagnostic(
                source_id,
                crate::diagnostics::DiagnosticCode("cem.ql.import_limit"),
                failure.message,
                "string import limit exceeded",
            ),
            ImportFailureKind::Unsupported | ImportFailureKind::Internal => ctx.fail_diagnostic(
                source_id,
                crate::diagnostics::DiagnosticCode("cem.ql.import_unsupported"),
                failure.message,
                "string import capability unavailable",
            ),
        },
    }
}
