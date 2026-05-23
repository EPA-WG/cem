use std::collections::BTreeSet;

use cem_ml::diagnostics::Severity;
use cem_ml::engine::FailLevel;
use cem_ml::report::ReportOptionsSnapshot;
use cem_ml::source::ByteRange;
use cem_ql::diagnostics::{
    self, ABORTED, BUDGET_EXCEEDED, CLOSURE_DETACHED, CROSS_TYPE_COMPARE, IMPORT_DENIED,
    IMPORT_UNRESOLVED, PARSE_ERROR, POLICY_ACCESSOR_FAILED, READ_DENIED, READ_DYNAMIC_ACCEPTS,
    READ_UNSATISFIABLE, RESERVED_SCHEME, SCOPE_VIOLATION, TYPE_ERROR, UNKNOWN_FUNCTION,
    UNKNOWN_TYPE, UNKNOWN_VARIABLE, UNRESOLVED_REFERENCE, USE_AND_OR,
};

#[test]
fn tier_a_diagnostic_table_matches_impl_section_8_codes() {
    let expected = [
        (PARSE_ERROR, Severity::Error),
        (USE_AND_OR, Severity::Error),
        (TYPE_ERROR, Severity::Error),
        (UNKNOWN_TYPE, Severity::Error),
        (UNKNOWN_FUNCTION, Severity::Error),
        (UNKNOWN_VARIABLE, Severity::Error),
        (SCOPE_VIOLATION, Severity::Error),
        (UNRESOLVED_REFERENCE, Severity::Warning),
        (CROSS_TYPE_COMPARE, Severity::Warning),
        (IMPORT_DENIED, Severity::Warning),
        (IMPORT_UNRESOLVED, Severity::Error),
        (RESERVED_SCHEME, Severity::Error),
        (READ_DENIED, Severity::Error),
        (READ_UNSATISFIABLE, Severity::Error),
        (READ_DYNAMIC_ACCEPTS, Severity::Warning),
        (ABORTED, Severity::Info),
        (BUDGET_EXCEEDED, Severity::Error),
        (CLOSURE_DETACHED, Severity::Info),
        (POLICY_ACCESSOR_FAILED, Severity::Error),
    ];

    let table = diagnostics::tier_a_diagnostics();
    assert_eq!(table.len(), expected.len());

    let mut seen = BTreeSet::new();
    for spec in table {
        assert!(spec.code.as_str().starts_with("cem.ql."), "{}", spec.code);
        assert!(seen.insert(spec.code.as_str()), "duplicate {}", spec.code);
        assert!(!spec.layer.is_empty(), "missing layer for {}", spec.code);
        assert!(
            !spec.description.is_empty(),
            "missing description for {}",
            spec.code
        );
        assert_eq!(
            diagnostics::lookup(spec.code.as_str()).unwrap().code,
            spec.code
        );
    }

    for (code, severity) in expected {
        let spec = diagnostics::lookup(code.as_str()).expect(code.as_str());
        assert_eq!(spec.default_severity, severity, "{code}");
        assert_eq!(diagnostics::default_severity(code), severity, "{code}");
    }
}

#[test]
fn query_diagnostics_route_through_cem_ml_report() {
    let diagnostics = vec![
        diagnostics::spanned_default(PARSE_ERROR, "bad query", ByteRange::new(0, 3)),
        diagnostics::spanned_default(CROSS_TYPE_COMPARE, "cross type", ByteRange::new(4, 2)),
        diagnostics::spanned_default(ABORTED, "aborted", ByteRange::new(7, 1)),
    ];

    let report = diagnostics::deterministic_report(
        vec!["query.cemql".to_owned()],
        diagnostics,
        ReportOptionsSnapshot {
            fail_level: FailLevel::Validate,
            schema: None,
            content_type: None,
            base_uri: None,
        },
    );

    assert_eq!(report.summary.input_count, 1);
    assert_eq!(report.summary.error_count, 1);
    assert_eq!(report.summary.warning_count, 1);
    assert_eq!(report.summary.info_count, 1);
    assert_eq!(report.summary.hard_violation_count, 1);
    assert_eq!(report.diagnostics[0].code, PARSE_ERROR.as_str());
    assert_eq!(report.diagnostics[0].byte_offset, Some(0));
}
