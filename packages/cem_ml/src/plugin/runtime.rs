//! Plugin runtime: invocation pipeline, observer / mutate gates,
//! source-map stitching, budgets, lifecycle plumbing
//! (AC-PL-3, AC-PL-9, AC-PL-10, AC-PL-12, AC-PL-13, AC-PL-15,
//! AC-PL-17, AC-PL-19).

use crate::diagnostics::Diagnostic;
use crate::plugin::chain::PluginChain;
use crate::plugin::descriptor::{
    AbortSignal, PluginContext, PluginDescriptor, PluginInput, PluginOutput, ScopeId,
};
use crate::plugin::errors::PluginError;
use crate::scheduler::policy::ScopePolicy;
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use std::time::{Duration, Instant};

/// Optional per-plugin resource limit (AC-PL-17). The runtime measures
/// wall-clock time around `invoke`; memory accounting is left to the
/// host. A failed budget surfaces as [`PluginError::Budget`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PluginBudget {
    pub time: Option<Duration>,
}

impl PluginBudget {
    pub fn time_ms(ms: u64) -> Self {
        Self {
            time: Some(Duration::from_millis(ms)),
        }
    }

    pub fn from_scope_policy(policy: &ScopePolicy) -> Self {
        Self {
            time: policy.plugin_time_budget_ms.map(Duration::from_millis),
        }
    }
}

/// Result of running the merged plugin chain over one input.
#[derive(Debug, Clone)]
pub struct PluginRunReport {
    pub output: PluginOutput,
    /// Plugin names that ran, in execution order (`observe` first,
    /// then `mutate`).
    pub executed: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

/// The Tier B plugin runtime. Stateless aside from the cooperatively
/// shared [`AbortSignal`] — each `invoke_chain` call drains the chain
/// once.
#[derive(Debug, Default)]
pub struct PluginRuntime {
    pub default_budget: PluginBudget,
}

impl PluginRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default_budget(budget: PluginBudget) -> Self {
        Self {
            default_budget: budget,
        }
    }

    /// Run the merged plugin chain (`observe` plugins first, then
    /// `mutate` plugins) for a single scope.
    ///
    /// `observe` plugins run sequentially in the supplied order and
    /// MUST NOT alter the bytes (AC-PL-3) — a violation surfaces as
    /// [`PluginError::ObserverViolation`] and aborts the chain
    /// (AC-PL-15).
    ///
    /// `mutate` plugins serialize (AC-PL-10) so each plugin's source
    /// map references the prior plugin's output; the runtime stitches
    /// the maps onto the inbound source map per AC-PL-13.
    pub fn invoke_chain(
        &self,
        chain: &PluginChain,
        input: PluginInput,
        scope: ScopeId,
        abort: AbortSignal,
        budget: PluginBudget,
    ) -> Result<PluginRunReport, PluginError> {
        self.invoke_chain_internal(chain, input, scope, abort, budget, None)
    }

    pub fn invoke_chain_with_control(
        &self,
        chain: &PluginChain,
        input: PluginInput,
        scope: ScopeId,
        control: &crate::operation_control::OperationControl,
        execution_scope: crate::operation_control::ExecutionScopeId,
        budget: PluginBudget,
    ) -> Result<PluginRunReport, PluginError> {
        self.invoke_chain_internal(
            chain,
            input,
            scope,
            control.abort_signal().clone(),
            budget,
            Some((control, execution_scope)),
        )
    }

    fn invoke_chain_internal(
        &self,
        chain: &PluginChain,
        input: PluginInput,
        scope: ScopeId,
        abort: AbortSignal,
        budget: PluginBudget,
        control: Option<(
            &crate::operation_control::OperationControl,
            crate::operation_control::ExecutionScopeId,
        )>,
    ) -> Result<PluginRunReport, PluginError> {
        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        let mut executed: Vec<String> = Vec::new();
        let mut current = PluginOutput {
            content_type: input.content_type.clone(),
            bytes: input.bytes.clone(),
            source_map: SourceMapStack::default(),
        };

        // Observe plugins (AC-PL-9 / AC-PL-10): see pre-mutation content.
        for plugin in chain.observe.iter() {
            ensure_plugin_active(&plugin.name, &abort, control)?;
            check_content_type(plugin, &current.content_type)?;
            let prior_bytes = current.bytes.clone();
            let prior_ct = current.content_type.clone();
            let mut ctx = PluginContext {
                scope,
                abort: abort.clone(),
                diagnostics: &mut diagnostics,
                inbound_source_map: current.source_map.clone(),
            };
            let output = invoke_with_budget(plugin, &as_input(&current), &mut ctx, budget)?;
            ensure_plugin_active(&plugin.name, &abort, control)?;
            if output.bytes != prior_bytes || output.content_type != prior_ct {
                return Err(PluginError::ObserverViolation {
                    plugin: plugin.name.clone(),
                });
            }
            executed.push(plugin.name.clone());
            // Observe-mode passes the existing source map through.
        }

        // Mutate plugins (AC-PL-10): serialized; each output's source
        // map is appended to the stitched chain.
        for plugin in chain.mutate.iter() {
            ensure_plugin_active(&plugin.name, &abort, control)?;
            check_content_type(plugin, &current.content_type)?;
            let inbound_map = current.source_map.clone();
            let mut ctx = PluginContext {
                scope,
                abort: abort.clone(),
                diagnostics: &mut diagnostics,
                inbound_source_map: inbound_map.clone(),
            };
            let mut output = invoke_with_budget(plugin, &as_input(&current), &mut ctx, budget)?;
            ensure_plugin_active(&plugin.name, &abort, control)?;
            // AC-PL-12 / AC-PL-13: stitch the plugin's emitted frame on
            // top of the inbound source map so a resolver can walk
            // back from the final output to the original source.
            output.source_map =
                stitched_source_map_for_scope(&inbound_map, &output.source_map, plugin, scope);
            current = output;
            executed.push(plugin.name.clone());
        }

        Ok(PluginRunReport {
            output: current,
            executed,
            diagnostics,
        })
    }

    /// Run a chain with the plugin budget inherited from the active
    /// scope policy (AC-A-4 / AC-PL-17).
    pub fn invoke_chain_with_policy(
        &self,
        chain: &PluginChain,
        input: PluginInput,
        scope: ScopeId,
        abort: AbortSignal,
        policy: &ScopePolicy,
    ) -> Result<PluginRunReport, PluginError> {
        self.invoke_chain(
            chain,
            input,
            scope,
            abort,
            PluginBudget::from_scope_policy(policy),
        )
    }
}

fn ensure_plugin_active(
    plugin: &str,
    abort: &AbortSignal,
    control: Option<(
        &crate::operation_control::OperationControl,
        crate::operation_control::ExecutionScopeId,
    )>,
) -> Result<(), PluginError> {
    if let Some((control, scope)) = control {
        match control.check_scope(scope) {
            Ok(()) => return Ok(()),
            Err(crate::operation_control::ControlError::Triggered(failure))
                if failure.terminal_class()
                    == crate::operation_control::ControlTerminalClass::Cancelled =>
            {
                return Err(PluginError::Cancelled {
                    plugin: plugin.to_owned(),
                });
            }
            Err(error) => {
                return Err(PluginError::Invoke {
                    plugin: plugin.to_owned(),
                    message: format!("operation control failure {}: {error}", error.code()),
                });
            }
        }
    }
    if abort.is_aborted() {
        Err(PluginError::Cancelled {
            plugin: plugin.to_owned(),
        })
    } else {
        Ok(())
    }
}

fn as_input(output: &PluginOutput) -> PluginInput {
    PluginInput {
        content_type: output.content_type.clone(),
        bytes: output.bytes.clone(),
    }
}

fn check_content_type(
    plugin: &PluginDescriptor,
    found: &crate::plugin::descriptor::ContentType,
) -> Result<(), PluginError> {
    if plugin.matches_input(found) {
        Ok(())
    } else {
        Err(PluginError::ContentTypeMismatch {
            plugin: plugin.name.clone(),
            expected: plugin
                .input_content_types
                .iter()
                .map(|c| c.as_str().to_owned())
                .collect(),
            found: found.as_str().to_owned(),
        })
    }
}

fn invoke_with_budget(
    plugin: &PluginDescriptor,
    input: &PluginInput,
    ctx: &mut PluginContext<'_>,
    budget: PluginBudget,
) -> Result<PluginOutput, PluginError> {
    let started = Instant::now();
    let result = plugin.invoke.invoke(input, ctx).map_err(|e| match e {
        PluginError::Invoke { .. } => e,
        PluginError::Cancelled { .. } => e,
        // Pass through plugin-emitted plugin errors verbatim. Unknown
        // engine errors get wrapped to keep the boundary clean.
        other => other,
    })?;
    if let Some(limit) = budget.time {
        let elapsed = started.elapsed();
        if elapsed > limit {
            return Err(PluginError::Budget {
                plugin: plugin.name.clone(),
                elapsed_ms: elapsed.as_millis(),
                budget_ms: limit.as_millis(),
            });
        }
    }
    Ok(result)
}

/// Stitch a plugin's emitted source map onto the inbound stack.
///
/// AC-PL-13. The composed stack starts with the inbound origin
/// frames, appends the plugin's own frames (if any), and tags the
/// boundary with a [`TransformKind::ContentTypeTransform`] frame
/// citing the plugin's output content type. Callers (and tests) can
/// walk `frames` first→last to retrace the chain from original source
/// to final output.
pub fn stitched_source_map(
    inbound: &SourceMapStack,
    emitted: &SourceMapStack,
    plugin: &PluginDescriptor,
) -> SourceMapStack {
    stitched_source_map_inner(inbound, emitted, plugin, None)
}

pub fn stitched_source_map_for_scope(
    inbound: &SourceMapStack,
    emitted: &SourceMapStack,
    plugin: &PluginDescriptor,
    scope: ScopeId,
) -> SourceMapStack {
    stitched_source_map_inner(inbound, emitted, plugin, Some(scope))
}

fn stitched_source_map_inner(
    inbound: &SourceMapStack,
    emitted: &SourceMapStack,
    plugin: &PluginDescriptor,
    scope: Option<ScopeId>,
) -> SourceMapStack {
    let mut stack = inbound.clone();
    // Boundary marker so observers can identify the plugin layer in
    // the trace. AC-PL-14 recommends recording the originating scope;
    // we add the plugin name into the content type tag here so the
    // serialized form remains a single string.
    let boundary_source_id = inbound
        .frames
        .last()
        .map(|f| f.source_id)
        .unwrap_or(crate::source::SourceId(0));
    stack.push(SourceMapFrame {
        source_id: boundary_source_id,
        span: FrameSpan::Single(ByteRange::new(0, 0)),
        transform: TransformKind::ContentTypeTransform {
            content_type: match scope {
                Some(scope) => format!(
                    "{}#{}@scope:{}",
                    plugin.output_content_type.as_str(),
                    plugin.name,
                    scope.0
                ),
                None => format!("{}#{}", plugin.output_content_type.as_str(), plugin.name),
            },
        },
    });
    for frame in &emitted.frames {
        stack.push(frame.clone());
    }
    stack
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::chain::{PluginChain, PluginRegistry};
    use crate::plugin::descriptor::{
        ContentType, PluginDescriptor, PluginEvidence, PluginInvoke, PluginMode,
    };
    use std::collections::BTreeSet;
    use std::sync::Arc;

    struct Identity;
    impl PluginInvoke for Identity {
        fn invoke(
            &self,
            input: &PluginInput,
            ctx: &mut PluginContext<'_>,
        ) -> Result<PluginOutput, PluginError> {
            Ok(PluginOutput {
                content_type: input.content_type.clone(),
                bytes: input.bytes.clone(),
                source_map: ctx.inbound_source_map.clone(),
            })
        }
    }

    struct ObserverThatMutates;
    impl PluginInvoke for ObserverThatMutates {
        fn invoke(
            &self,
            input: &PluginInput,
            _: &mut PluginContext<'_>,
        ) -> Result<PluginOutput, PluginError> {
            let mut bytes = input.bytes.clone();
            bytes.push(b'!');
            Ok(PluginOutput {
                content_type: input.content_type.clone(),
                bytes,
                source_map: SourceMapStack::default(),
            })
        }
    }

    struct CancelsDuringInvoke;
    impl PluginInvoke for CancelsDuringInvoke {
        fn invoke(
            &self,
            input: &PluginInput,
            ctx: &mut PluginContext<'_>,
        ) -> Result<PluginOutput, PluginError> {
            ctx.abort.abort();
            Ok(PluginOutput {
                content_type: input.content_type.clone(),
                bytes: input.bytes.clone(),
                source_map: ctx.inbound_source_map.clone(),
            })
        }
    }

    struct SlowIdentity;
    impl PluginInvoke for SlowIdentity {
        fn invoke(
            &self,
            input: &PluginInput,
            ctx: &mut PluginContext<'_>,
        ) -> Result<PluginOutput, PluginError> {
            std::thread::sleep(std::time::Duration::from_millis(3));
            Ok(PluginOutput {
                content_type: input.content_type.clone(),
                bytes: input.bytes.clone(),
                source_map: ctx.inbound_source_map.clone(),
            })
        }
    }

    fn observer(name: &str, invoke: Arc<dyn PluginInvoke>) -> Arc<PluginDescriptor> {
        Arc::new(PluginDescriptor {
            name: name.into(),
            version: "0.1".into(),
            input_content_types: vec!["text/css".into()],
            output_content_type: "text/css".into(),
            mode: PluginMode::Observe,
            supports_source_map: false,
            priority: 0,
            requires: BTreeSet::new(),
            evidence: PluginEvidence::empty(),
            invoke,
        })
    }

    #[test]
    fn observer_violation_surfaces_when_observe_mutates() {
        let plugin = observer("naughty", Arc::new(ObserverThatMutates));
        let mut local = PluginRegistry::new();
        local.install(plugin).unwrap();
        let chain = PluginChain::merged(&[], &local, &ContentType::from("text/css"));
        let runtime = PluginRuntime::new();
        let err = runtime
            .invoke_chain(
                &chain,
                PluginInput::new("text/css", b"body{}".to_vec()),
                ScopeId(0),
                AbortSignal::new(),
                PluginBudget::default(),
            )
            .unwrap_err();
        assert!(matches!(err, PluginError::ObserverViolation { .. }));
    }

    #[test]
    fn cancellation_short_circuits_chain() {
        let plugin = observer("ident", Arc::new(Identity));
        let mut local = PluginRegistry::new();
        local.install(plugin).unwrap();
        let chain = PluginChain::merged(&[], &local, &ContentType::from("text/css"));
        let runtime = PluginRuntime::new();
        let abort = AbortSignal::new();
        abort.abort();
        let err = runtime
            .invoke_chain(
                &chain,
                PluginInput::new("text/css", b"body{}".to_vec()),
                ScopeId(0),
                abort,
                PluginBudget::default(),
            )
            .unwrap_err();
        assert!(matches!(err, PluginError::Cancelled { .. }));
    }

    #[test]
    fn cancellation_during_plugin_invocation_suppresses_its_output() {
        let plugin = observer("cancelling", Arc::new(CancelsDuringInvoke));
        let mut local = PluginRegistry::new();
        local.install(plugin).unwrap();
        let chain = PluginChain::merged(&[], &local, &ContentType::from("text/css"));
        let err = PluginRuntime::new()
            .invoke_chain(
                &chain,
                PluginInput::new("text/css", b"body{}".to_vec()),
                ScopeId(0),
                AbortSignal::new(),
                PluginBudget::default(),
            )
            .expect_err("cancelled plugin output must be suppressed");
        assert!(matches!(err, PluginError::Cancelled { .. }));
    }

    #[test]
    fn deadline_during_plugin_invocation_suppresses_its_output() {
        let plugin = observer("slow", Arc::new(SlowIdentity));
        let mut local = PluginRegistry::new();
        local.install(plugin).unwrap();
        let chain = PluginChain::merged(&[], &local, &ContentType::from("text/css"));
        let control = crate::operation_control::OperationControl::with_root_policy(
            AbortSignal::new(),
            ScopePolicy::host_root().with_timeout_ms(Some(1)),
        )
        .unwrap();
        let error = PluginRuntime::new()
            .invoke_chain_with_control(
                &chain,
                PluginInput::new("text/css", b"body{}".to_vec()),
                ScopeId(0),
                &control,
                crate::operation_control::ROOT_EXECUTION_SCOPE_ID,
                PluginBudget::default(),
            )
            .expect_err("a plugin result arriving after its deadline must be suppressed");
        assert!(matches!(
            error,
            PluginError::Invoke { ref message, .. }
                if message.contains("timeout-exceeded")
        ));
    }

    #[test]
    fn plugin_abort_signal_is_the_canonical_scheduler_signal() {
        let scheduler_signal = crate::scheduler::AbortSignal::new();
        let plugin_signal: crate::plugin::AbortSignal = scheduler_signal.clone();
        scheduler_signal.abort();
        assert!(plugin_signal.is_aborted());
    }
}
