//! Bounded native value traversal at explicit projection/export boundaries.
use super::*;
use cem_ml::operation_control::{
    ControlCause, ControlError, ExecutionScopeId, MemoryPermit, OperationControl,
};
use cem_ml::value::artifact::CemValueArtifactLimits;

pub struct ValueControl<'a> {
    pub control: &'a OperationControl,
    pub scope: ExecutionScopeId,
    pub query_scope: QueryContextScope,
    pub limits: CemValueArtifactLimits,
    work: usize,
    bytes: usize,
    permits: Vec<MemoryPermit>,
}
impl<'a> ValueControl<'a> {
    pub fn new(
        control: &'a OperationControl,
        scope: ExecutionScopeId,
        query_scope: QueryContextScope,
        mut limits: CemValueArtifactLimits,
    ) -> Result<Self, ControlError> {
        control.check_scope(scope)?;
        let tree = control.scope_tree();
        let policy = tree
            .scope(scope)
            .ok_or(ControlError::UnknownScope(scope))?
            .effective_policy;
        limits.max_bytes = limits
            .max_bytes
            .min(policy.memory_bytes.try_into().unwrap_or(usize::MAX));
        limits.max_depth = limits.max_depth.min(policy.stack_depth as usize);
        Ok(Self {
            control,
            scope,
            query_scope,
            limits,
            work: 0,
            bytes: 0,
            permits: Vec::new(),
        })
    }
    pub fn into_permits(self) -> Vec<MemoryPermit> {
        self.permits
    }
    pub fn failure(&self, code: &str) -> ControlError {
        match self.control.fail_scope(
            self.scope,
            ControlCause::InternalFailure {
                diagnostic_code: code.into(),
            },
            None,
        ) {
            Ok(failure) => ControlError::Triggered(failure),
            Err(error) => error,
        }
    }
    pub fn charge(&mut self, bytes: usize, depth: usize) -> Result<(), ControlError> {
        self.control.check_scope(self.scope)?;
        self.work = self.work.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes);
        if self.work > self.limits.max_values
            || self.bytes > self.limits.max_bytes
            || depth > self.limits.max_depth
        {
            return Err(self.failure("cem.value.limit"));
        }
        if bytes > 0 {
            self.permits
                .push(self.control.charge_memory(self.scope, bytes as u64, None)?);
        }
        Ok(())
    }
    pub fn text(&mut self, items: &[Item]) -> Result<String, ControlError> {
        let mut result = String::new();
        let mut pending = vec![items.iter()];
        while let Some(items) = pending.last_mut() {
            let Some(item) = items.next() else {
                pending.pop();
                continue;
            };
            self.charge(0, pending.len())?;
            if let Some(targets) = values::reference_values(item) {
                pending.push(targets.iter());
                continue;
            }
            if let Some(view) = item.view().filter(|v| v.kind() == QueryItemViewKind::Node) {
                let fragments = view
                    .text_segments(self.query_scope)
                    .map_err(|e| self.access_error(e))?;
                for fragment in fragments {
                    let fragment = fragment.map_err(|e| self.access_error(e))?;
                    self.charge(fragment.len(), pending.len())?;
                    result.push_str(&fragment);
                }
            } else if let Some(atom) = item.atom() {
                let lexical = crate::render::item_to_string(&Item::Atomic(atom));
                self.charge(lexical.len(), pending.len())?;
                result.push_str(&lexical);
            } else {
                return Err(self.failure("cem.value.text_unsupported"));
            }
        }
        self.control.check_scope(self.scope)?;
        Ok(result)
    }
    pub fn access_error(&self, error: QueryNodeAccessError) -> ControlError {
        self.failure(match error {
            QueryNodeAccessError::ScopeViolation => "cem.ql.scope_violation",
            QueryNodeAccessError::Unsupported => "cem.ql.unsupported",
        })
    }
}
