//! CEM-QL diagnostic code table.

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::report::{Report, ReportOptionsSnapshot};
use cem_ml::source::ByteRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagnosticCode(pub &'static str);

impl DiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl From<DiagnosticCode> for String {
    fn from(code: DiagnosticCode) -> Self {
        code.as_str().to_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticSpec {
    pub code: DiagnosticCode,
    pub default_severity: Severity,
    pub layer: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct QueryDiagnostic {
    pub code: DiagnosticCode,
    pub diagnostic: Diagnostic,
}

pub const PARSE_ERROR: DiagnosticCode = DiagnosticCode("cem.ql.parse_error");
pub const USE_AND_OR: DiagnosticCode = DiagnosticCode("cem.ql.use_and_or");
pub const TYPE_ERROR: DiagnosticCode = DiagnosticCode("cem.ql.type_error");
pub const UNKNOWN_TYPE: DiagnosticCode = DiagnosticCode("cem.ql.unknown_type");
pub const UNKNOWN_FUNCTION: DiagnosticCode = DiagnosticCode("cem.ql.unknown_function");
pub const UNKNOWN_VARIABLE: DiagnosticCode = DiagnosticCode("cem.ql.unknown_variable");
pub const SCOPE_VIOLATION: DiagnosticCode = DiagnosticCode("cem.ql.scope_violation");
pub const UNRESOLVED_REFERENCE: DiagnosticCode = DiagnosticCode("cem.ql.unresolved_reference");
pub const CROSS_TYPE_COMPARE: DiagnosticCode = DiagnosticCode("cem.ql.cross_type_compare");
pub const IMPORT_DENIED: DiagnosticCode = DiagnosticCode("cem.ql.import_denied");
pub const IMPORT_UNRESOLVED: DiagnosticCode = DiagnosticCode("cem.ql.import_unresolved");
pub const RESERVED_SCHEME: DiagnosticCode = DiagnosticCode("cem.ql.reserved_scheme");
pub const READ_DENIED: DiagnosticCode = DiagnosticCode("cem.ql.read_denied");
pub const READ_UNSATISFIABLE: DiagnosticCode = DiagnosticCode("cem.ql.read_unsatisfiable");
pub const READ_DYNAMIC_ACCEPTS: DiagnosticCode = DiagnosticCode("cem.ql.read_dynamic_accepts");
pub const ABORTED: DiagnosticCode = DiagnosticCode("cem.ql.aborted");
pub const BUDGET_EXCEEDED: DiagnosticCode = DiagnosticCode("cem.ql.budget_exceeded");
pub const CLOSURE_DETACHED: DiagnosticCode = DiagnosticCode("cem.ql.closure_detached");
pub const POLICY_ACCESSOR_FAILED: DiagnosticCode = DiagnosticCode("cem.ql.policy_accessor_failed");

pub const TIER_A_DIAGNOSTICS: &[DiagnosticSpec] = &[
    DiagnosticSpec {
        code: PARSE_ERROR,
        default_severity: Severity::Error,
        layer: "L1 / L2",
        description: "Lexer or parser failed; range = offending tokens.",
    },
    DiagnosticSpec {
        code: USE_AND_OR,
        default_severity: Severity::Error,
        layer: "L1",
        description: "`&&` / `||` reserved; suggest `and`/`or`.",
    },
    DiagnosticSpec {
        code: TYPE_ERROR,
        default_severity: Severity::Error,
        layer: "L4 / L6",
        description: "Static or runtime type failure.",
    },
    DiagnosticSpec {
        code: UNKNOWN_TYPE,
        default_severity: Severity::Error,
        layer: "L4",
        description: "Type name not in active schema.",
    },
    DiagnosticSpec {
        code: UNKNOWN_FUNCTION,
        default_severity: Severity::Error,
        layer: "L3",
        description: "Function name not in resolution chain.",
    },
    DiagnosticSpec {
        code: UNKNOWN_VARIABLE,
        default_severity: Severity::Error,
        layer: "L3",
        description: "Variable name not in resolution chain.",
    },
    DiagnosticSpec {
        code: SCOPE_VIOLATION,
        default_severity: Severity::Error,
        layer: "L6",
        description: "Access outside QueryContextScope.",
    },
    DiagnosticSpec {
        code: UNRESOLVED_REFERENCE,
        default_severity: Severity::Warning,
        layer: "L6",
        description: "Reference slot unresolved; scope policy may raise to error.",
    },
    DiagnosticSpec {
        code: CROSS_TYPE_COMPARE,
        default_severity: Severity::Warning,
        layer: "L4",
        description: "Cross-atom-type comparison; silenced under dev profile.",
    },
    DiagnosticSpec {
        code: IMPORT_DENIED,
        default_severity: Severity::Warning,
        layer: "L3",
        description: "Scope policy denied network-scheme import.",
    },
    DiagnosticSpec {
        code: IMPORT_UNRESOLVED,
        default_severity: Severity::Error,
        layer: "L3",
        description: "`urn:cem:` URI not registered.",
    },
    DiagnosticSpec {
        code: RESERVED_SCHEME,
        default_severity: Severity::Error,
        layer: "policy load",
        description: "Scope policy attempted to grant `cem:` / `urn:cem:`.",
    },
    DiagnosticSpec {
        code: READ_DENIED,
        default_severity: Severity::Error,
        layer: "L6",
        description: "`read()` URI denied by scope policy.",
    },
    DiagnosticSpec {
        code: READ_UNSATISFIABLE,
        default_severity: Severity::Error,
        layer: "L6",
        description: "`read()` content type has no transform to a resolved accepts entry.",
    },
    DiagnosticSpec {
        code: READ_DYNAMIC_ACCEPTS,
        default_severity: Severity::Warning,
        layer: "L4",
        description: "`read()` accepts argument is dynamic; binary stamps as wildcard.",
    },
    DiagnosticSpec {
        code: ABORTED,
        default_severity: Severity::Info,
        layer: "L6",
        description: "Evaluation aborted via AbortSignal.",
    },
    DiagnosticSpec {
        code: BUDGET_EXCEEDED,
        default_severity: Severity::Error,
        layer: "L6",
        description: "Scope-policy budget breached; carries limit name.",
    },
    DiagnosticSpec {
        code: CLOSURE_DETACHED,
        default_severity: Severity::Info,
        layer: "L5 / L6",
        description: "Closure capture detached host-AST refs.",
    },
    DiagnosticSpec {
        code: POLICY_ACCESSOR_FAILED,
        default_severity: Severity::Error,
        layer: "L6",
        description: "Policy-supplied resource accessor returned an error.",
    },
];

pub fn tier_a_diagnostics() -> &'static [DiagnosticSpec] {
    TIER_A_DIAGNOSTICS
}

pub fn lookup(code: impl AsRef<str>) -> Option<&'static DiagnosticSpec> {
    let code = code.as_ref();
    TIER_A_DIAGNOSTICS
        .iter()
        .find(|spec| spec.code.as_str() == code)
}

pub fn default_severity(code: DiagnosticCode) -> Severity {
    lookup(code.as_str())
        .map(|spec| spec.default_severity)
        .unwrap_or(Severity::Error)
}

pub fn spanned(
    code: DiagnosticCode,
    message: impl Into<String>,
    range: ByteRange,
    severity: Severity,
) -> Diagnostic {
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: Some(range.start),
        code: code.into(),
        severity,
        message: message.into(),
        node: None,
        source_map: None,
    }
}

pub fn spanned_default(
    code: DiagnosticCode,
    message: impl Into<String>,
    range: ByteRange,
) -> Diagnostic {
    spanned(code, message, range, default_severity(code))
}

pub fn deterministic_report(
    inputs: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    options: ReportOptionsSnapshot,
) -> Report {
    Report::deterministic(inputs, diagnostics, options)
}
