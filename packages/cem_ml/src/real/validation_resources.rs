//! Coordinator-side suspension driver. No queue submission or join happens on a
//! physical CPU worker. Native session ownership moves through CPU continuations.
use super::{
    native_scheduler_error, time_budget_diagnostics, EngineContext, EngineError, EngineResult,
    PendingScheduledValidation, ScheduledValidationOutcome,
};
use crate::{
    diagnostics::{Diagnostic, Severity},
    operation_control::MemoryPermit,
    scheduler::{NativeScheduler, ScheduledTaskSpec, TaskPath},
    schema::input_validation::resumable::{InputValidationResourceCompletion, InputValidationRun},
};

pub(super) fn finish(
    scheduler: &NativeScheduler,
    context: &EngineContext,
    index: u32,
    mut outcome: ScheduledValidationOutcome,
) -> EngineResult<ScheduledValidationOutcome> {
    let mut round = 0_u32;
    while let Some(pending) = outcome.pending.take() {
        context.ensure_active()?;
        let PendingScheduledValidation {
            mut validation,
            started_at,
            input,
            budget_aliases,
        } = pending;
        let execution = validation.request.execution.clone();
        scheduler
            .control()
            .check_scope(execution.scope)
            .map_err(|e| native_scheduler_error(e.into()))?;
        let path = TaskPath::root(index)
            .child(2)
            .and_then(|p| p.child(round))
            .map_err(native_scheduler_error)?;
        let mut reads = Vec::with_capacity(validation.resources.len());
        for (resource_index, resource) in std::mem::take(&mut validation.resources)
            .into_iter()
            .enumerate()
        {
            let registry = context.resolver_registry.clone();
            let control = execution.control.clone();
            let scope = execution.scope;
            let io_path = path
                .child(0)
                .and_then(|p| {
                    p.child(
                        u32::try_from(resource_index)
                            .map_err(|_| crate::scheduler::ScheduleError::InvalidTaskLabel)?,
                    )
                })
                .map_err(native_scheduler_error)?;
            reads.push(scheduler.submit_io(
                ScheduledTaskSpec::new(scope, io_path, format!("{}:schema-resource:{}", input.uri, resource.id)).with_trace_scope(index),
                move || -> EngineResult<(InputValidationResourceCompletion, Option<MemoryPermit>)> {
                    let mut result = resource.resource.read(&registry, &control, scope);
                    // Cancellation/budget expiry after transport suppresses all
                    // subsequent import and runtime-context callbacks.
                    control.check_scope(scope).map_err(|e| native_scheduler_error(e.into()))?;
                    if let Ok(response) = &result {
                        if response.bytes.len() > crate::import::MAX_DOCUMENT_BYTES {
                            result = Err(Diagnostic {
                                code: "cem.schema.import_failed".into(), severity: Severity::Error,
                                uri: Some(response.uri.clone()), message: "Source exceeds the 16 MiB document import limit.".into(), source_map: Some(resource.resource.source_map().clone()), ..Default::default()
                            });
                        }
                    }
                    let permit = match &result {
                        Ok(response) => Some(control.charge_memory(scope, response.bytes.len() as u64, Some(resource.resource.source_map().clone())).map_err(|e| native_scheduler_error(e.into()))?),
                        Err(_) => None,
                    };
                    Ok((InputValidationResourceCompletion { id: resource.id, result }, permit))
                },
            ).map_err(native_scheduler_error)?);
        }
        let mut completions = Vec::with_capacity(reads.len());
        let mut permits = Vec::with_capacity(reads.len());
        for read in reads {
            let (completion, permit) = read.join().map_err(native_scheduler_error)?.value?;
            completions.push(completion);
            permits.extend(permit);
        }
        let control = execution.control.clone();
        let scope = execution.scope;
        let resume = scheduler
            .submit_cpu(
                ScheduledTaskSpec::new(
                    scope,
                    path.child(1).map_err(native_scheduler_error)?,
                    format!("{}:validation-resume:{round}", input.uri),
                )
                .with_trace_scope(index),
                move || -> EngineResult<InputValidationRun> {
                    control
                        .check_scope(scope)
                        .map_err(|e| native_scheduler_error(e.into()))?;
                    let result = validation.resume(completions, permits);
                    control
                        .check_scope(scope)
                        .map_err(|e| native_scheduler_error(e.into()))?;
                    Ok(result)
                },
            )
            .map_err(native_scheduler_error)?;
        let result = resume.join().map_err(native_scheduler_error)?.value?;
        outcome.diagnostics.extend(result.diagnostics);
        if let Some(validation) = result.pending {
            outcome.pending = Some(PendingScheduledValidation {
                validation,
                started_at,
                input,
                budget_aliases,
            });
        } else {
            for completion in &mut outcome.completion {
                completion.complete = result.complete;
            }
            let aliases: Vec<_> = budget_aliases.iter().map(String::as_str).collect();
            let mut timing = time_budget_diagnostics(
                &input.root_scope,
                &aliases,
                started_at.elapsed().as_nanos(),
            );
            super::project_diagnostic_uris(&mut timing, &input, context);
            outcome.diagnostics.extend(timing);
            let _ = scheduler.control().complete_scope(scope);
        }
        round = round.checked_add(1).ok_or_else(|| {
            EngineError::Internal("validation session round limit exceeded".into())
        })?;
    }
    Ok(outcome)
}
