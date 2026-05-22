//! CEM-QL diagnostic code shell.

use cem_ml::diagnostics::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(pub &'static str);

#[derive(Debug, Clone)]
pub struct QueryDiagnostic {
    pub code: DiagnosticCode,
    pub diagnostic: Diagnostic,
}
