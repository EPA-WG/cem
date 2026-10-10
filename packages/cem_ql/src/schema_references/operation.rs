//! Explicit host operation shared by selector evaluation and lifecycle gates.
use super::CemQlSchemaDeclarationHost;
use cem_ml::{
    operation_control::{ExecutionScopeId, OperationControl},
    value::reference_resolution::ReferenceResolutionError,
};

impl CemQlSchemaDeclarationHost {
    /// Bind this execution to an existing operation. Replacing the operation
    /// expires prepared namespace proofs. Source captures and grants stay owned
    /// by the caller; a stopped operation cannot activate or publish a result.
    pub fn set_operation_control(&mut self, control: OperationControl, scope: ExecutionScopeId) {
        self.operation = Some((control, scope));
        self.invalidate_namespace_publications();
    }

    pub(super) fn check_operation(&self) -> Result<(), ReferenceResolutionError> {
        match &self.operation {
            Some((control, scope)) => control
                .check_scope(*scope)
                .map_err(|_| ReferenceResolutionError::OperationStopped),
            None => Ok(()),
        }
    }
}
