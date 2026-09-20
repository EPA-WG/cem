//! Inert presentation of a retained CEM document; external syntax stays in import.
use super::{AtomValue, EvalCtx, Item, ItemStream};
use crate::{diagnostics::DiagnosticCode, ir::IrId};
use cem_ml::{
    conversion::{
        direct_cem_output_pipeline,
        execute_conversion_output_pipeline_from_cem_tree_with_environment,
        ConversionOutputPipelineEnvironment, ConversionOutputPipelineExecution, ConversionRegistry,
    },
    operation_control::MemoryPermit,
    parser::CemAstNode,
    projection::cem_tree_inspection,
    schema::SchemaRegistry,
};
use std::sync::Arc;

pub(super) fn inspect(
    arguments: Vec<ItemStream>,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> ItemStream {
    let document = match arguments[0].items.as_slice() {
        [] => return ItemStream::empty(),
        [item] => super::data::retained_document(item),
        _ => None,
    };
    let Some(owner) = document else {
        return ctx.type_error(
            source,
            "cemml:inspect requires zero or one retained document",
        );
    };
    if let Err(failure) = ctx.force_safe_point(source) {
        return failure;
    }
    if let Err(failure) = ctx.charge_items(owner.ast().nodes.len() as u64, source) {
        return failure;
    }
    // Bound the source payload before projection duplicates it. This inspects
    // only the common CEM arena; no source decoding or parser-owner traversal.
    let mut payload_bytes = owner.source_uri().len() as u64;
    for node in &owner.ast().nodes {
        if let Err(failure) = ctx.poll_work(source) {
            return failure;
        }
        payload_bytes = payload_bytes.saturating_add(node_payload_bytes(node));
        if let Err(failure) = check_payload_limit(payload_bytes, ctx, source) {
            return failure;
        }
    }
    let _payload_permit = match charge_payload(payload_bytes, ctx, source) {
        Ok(permit) => permit,
        Err(failure) => return failure,
    };
    let stream = Arc::new(cem_tree_inspection(owner.clone()));
    if let Err(failure) = ctx.force_safe_point(source) {
        return failure;
    }
    let registry = SchemaRegistry::with_builtin_schemas();
    let conversions = ConversionRegistry::with_builtin_converters();
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &registry,
        conversion_registry: &conversions,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let mut pipeline = direct_cem_output_pipeline();
    pipeline.cemt_options.formatter_profile = Some("tabular".into());
    pipeline.cemt_insertion_context.formatter_profile = Some("tabular".into());
    pipeline.writer_insertion_context.formatter_profile = Some("tabular".into());
    let result = execute_conversion_output_pipeline_from_cem_tree_with_environment(
        &environment,
        &pipeline,
        stream,
        owner.node(0).map(|node| node.source.clone()),
        vec![],
        "cemml:inspect",
        None,
        Some(owner.source_uri()),
    );
    // Native formatting is synchronous. Observe cancellation/deadlines after
    // it as well as before it; its text remains private until acceptance.
    if let Err(failure) = ctx.force_safe_point(source) {
        return failure;
    }
    accept_output(result, ctx, source)
}

fn node_payload_bytes(node: &CemAstNode) -> u64 {
    use CemAstNode::*;
    let parts: &[&str] = match node {
        Document { .. } => &[],
        Element { expanded_name, .. } => &[&expanded_name.local_name, &expanded_name.namespace_uri],
        Attribute {
            expanded_name,
            value,
            ..
        } => &[
            &expanded_name.local_name,
            &expanded_name.namespace_uri,
            value.as_deref().unwrap_or(""),
        ],
        Text { data, .. }
        | Whitespace { data, .. }
        | Comment { data, .. }
        | Cdata { data, .. }
        | RawText { data, .. } => &[data],
        ProcessingInstruction { target, data, .. } => &[target, data],
        Error { code, .. } => &[code],
    };
    parts
        .iter()
        .fold(0u64, |size, part| size.saturating_add(part.len() as u64))
}

fn check_payload_limit(bytes: u64, ctx: &mut EvalCtx<'_>, source: IrId) -> Result<(), ItemStream> {
    if bytes > ctx.scope_policy.memory_bytes {
        return Err(ctx.fail_diagnostic(
            source,
            DiagnosticCode("cem.ql.inspect_limit"),
            format!(
                "inspection payload {bytes} bytes exceeds scope limit {}",
                ctx.scope_policy.memory_bytes
            ),
            "inspection payload limit exceeded",
        ));
    }
    Ok(())
}

fn charge_payload(
    bytes: u64,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> Result<MemoryPermit, ItemStream> {
    check_payload_limit(bytes, ctx, source)?;
    match ctx
        .control
        .charge_memory(ctx.safe_points.scope(), bytes, Some(ctx.source_map(source)))
    {
        Ok(permit) => Ok(permit),
        Err(error) => {
            ctx.map_control_result(source, Err(error))?;
            unreachable!("a control error always produces a failed stream")
        }
    }
}

fn accept_output(
    result: ConversionOutputPipelineExecution,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> ItemStream {
    // A final text-output envelope is the public writer boundary, never a tree
    // projection or a serialization/reparse handoff between execution stages.
    if result
        .diagnostics
        .iter()
        .any(|d| d.severity == cem_ml::diagnostics::Severity::Error)
    {
        ctx.diagnostics.extend(result.diagnostics);
        return ctx.fail_diagnostic(
            source,
            DiagnosticCode("cem.ql.inspect_writer"),
            "typed inspection writer failed",
            "typed inspection writer failed",
        );
    }
    let Some(serde_json::Value::String(text)) = result.output else {
        return ctx.fail_diagnostic(
            source,
            DiagnosticCode("cem.ql.inspect_writer"),
            "typed inspection writer did not produce display text",
            "typed inspection writer failed",
        );
    };
    let _output_permit = match charge_payload(text.len() as u64, ctx, source) {
        Ok(permit) => permit,
        Err(failure) => return failure,
    };
    ctx.diagnostics.extend(result.diagnostics);
    ItemStream::once(Item::Atomic(AtomValue::String(text)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{compile, CompileContext, EvaluationContext};
    use cem_ml::{
        diagnostics::{Diagnostic, Severity},
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    };

    #[test]
    fn writer_errors_or_missing_text_discard_all_output() {
        let query = compile("1", &CompileContext::default()).unwrap();
        let context = EvaluationContext::default();
        let control = OperationControl::default();
        for result in [
            ConversionOutputPipelineExecution::default(),
            ConversionOutputPipelineExecution {
                output: Some(serde_json::Value::String("partial".into())),
                diagnostics: vec![Diagnostic {
                    severity: Severity::Error,
                    message: "writer failure".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ] {
            let mut ctx = EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
            let failure = accept_output(result, &mut ctx, query.tree.root);
            assert!(failure.error.is_some());
            assert!(failure.items.is_empty());
            assert!(failure
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.ql.inspect_writer" && d.source_map.is_some()));
        }
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }

    #[test]
    fn cancellation_at_writer_acceptance_discards_completed_text() {
        let query = compile("1", &CompileContext::default()).unwrap();
        let context = EvaluationContext::default();
        let control = OperationControl::default();
        let mut ctx = EvalCtx::new(&query, &context, &control, ROOT_EXECUTION_SCOPE_ID);
        control.cancel_root(None, None).unwrap();
        let failure = accept_output(
            ConversionOutputPipelineExecution {
                output: Some(serde_json::Value::String("completed but unaccepted".into())),
                ..Default::default()
            },
            &mut ctx,
            query.tree.root,
        );
        assert_eq!(failure.error, Some(super::super::EvalError::Cancelled));
        assert!(failure.items.is_empty());
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
}
